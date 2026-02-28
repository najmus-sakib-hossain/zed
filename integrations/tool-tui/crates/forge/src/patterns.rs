//! LSP Pattern Detection for DX Tools
//!
//! Detects dx-tool patterns in source code:
//! - dxButton, dxInput, dxCard (dx-ui components)
//! - dxiHome, dxiUser, dxiSettings (dx-icons)
//! - dxfRoboto, dxfInter (dx-fonts)
//! - dxaGoogleLogin (dx-auth)

use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Pattern match result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMatch {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub pattern: String,
    pub tool: DxToolType,
    pub component_name: String,
}

/// DX Tool type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DxToolType {
    Ui,    // dx-ui
    Icons, // dx-icons
    Fonts, // dx-fonts
    Style, // dx-style
    I18n,  // dx-i18n
    Auth,  // dx-auth
    Check, // dx-check
    Custom(String),
}

impl DxToolType {
    pub fn prefix(&self) -> &str {
        match self {
            DxToolType::Ui => "dx",
            DxToolType::Icons => "dxi",
            DxToolType::Fonts => "dxf",
            DxToolType::Style => "sr",
            DxToolType::I18n => "dxt",
            DxToolType::Auth => "dxa",
            DxToolType::Check => "dxc",
            DxToolType::Custom(prefix) => prefix,
        }
    }

    pub fn tool_name(&self) -> &str {
        match self {
            DxToolType::Ui => "dx-ui",
            DxToolType::Icons => "dx-icons",
            DxToolType::Fonts => "dx-fonts",
            DxToolType::Style => "dx-style",
            DxToolType::I18n => "dx-i18n",
            DxToolType::Auth => "dx-auth",
            DxToolType::Check => "dx-check",
            DxToolType::Custom(name) => name,
        }
    }

    pub fn from_prefix(prefix: &str) -> Self {
        match prefix {
            "dx" => DxToolType::Ui,
            "dxi" => DxToolType::Icons,
            "dxf" => DxToolType::Fonts,
            "sr" => DxToolType::Style,
            "dxt" => DxToolType::I18n,
            "dxa" => DxToolType::Auth,
            "dxc" => DxToolType::Check,
            other => DxToolType::Custom(other.to_string()),
        }
    }
}

/// Pattern detector for DX tool references
pub struct PatternDetector {
    patterns: HashMap<DxToolType, Regex>,
}

impl PatternDetector {
    /// Create a new pattern detector
    pub fn new() -> Result<Self> {
        let mut patterns = HashMap::new();

        // dx-ui: dxButton, dxInput, dxCard, etc.
        patterns.insert(DxToolType::Ui, Regex::new(r"\bdx([A-Z][a-zA-Z0-9]*)\b")?);

        // dx-icons: dxiHome, dxiUser, dxiSettings, etc.
        patterns.insert(DxToolType::Icons, Regex::new(r"\bdxi([A-Z][a-zA-Z0-9]*)\b")?);

        // dx-fonts: dxfRoboto, dxfInter, dxfPoppins, etc.
        patterns.insert(DxToolType::Fonts, Regex::new(r"\bdxf([A-Z][a-zA-Z0-9]*)\b")?);

        // dx-style: dxsContainer, dxsFlex, etc.
        patterns.insert(DxToolType::Style, Regex::new(r"\bdxs([A-Z][a-zA-Z0-9]*)\b")?);

        // dx-i18n: dxtText, dxtMessage, etc.
        patterns.insert(DxToolType::I18n, Regex::new(r"\bdxt([A-Z][a-zA-Z0-9]*)\b")?);

        // dx-auth: dxaGoogleLogin, dxaGithubLogin, etc.
        patterns.insert(DxToolType::Auth, Regex::new(r"\bdxa([A-Z][a-zA-Z0-9]*)\b")?);

        Ok(Self { patterns })
    }

    /// Detect patterns in a file
    pub fn detect_in_file(&self, path: &Path, content: &str) -> Result<Vec<PatternMatch>> {
        let mut matches = Vec::new();

        for (line_idx, line) in content.lines().enumerate() {
            for (tool, regex) in &self.patterns {
                for cap in regex.captures_iter(line) {
                    if let Some(m) = cap.get(0) {
                        let component_name = cap.get(1).map(|c| c.as_str()).unwrap_or("");

                        matches.push(PatternMatch {
                            file: path.to_path_buf(),
                            line: line_idx + 1,
                            column: m.start() + 1,
                            pattern: m.as_str().to_string(),
                            tool: tool.clone(),
                            component_name: component_name.to_string(),
                        });
                    }
                }
            }
        }

        Ok(matches)
    }

    /// Detect patterns in multiple files
    pub fn detect_in_files(&self, files: &[(PathBuf, String)]) -> Result<Vec<PatternMatch>> {
        let mut all_matches = Vec::new();

        for (path, content) in files {
            let matches = self.detect_in_file(path, content)?;
            all_matches.extend(matches);
        }

        Ok(all_matches)
    }

    /// Group matches by tool type
    pub fn group_by_tool(
        &self,
        matches: Vec<PatternMatch>,
    ) -> HashMap<DxToolType, Vec<PatternMatch>> {
        let mut grouped: HashMap<DxToolType, Vec<PatternMatch>> = HashMap::new();

        for m in matches {
            grouped.entry(m.tool.clone()).or_default().push(m);
        }

        grouped
    }

    /// Check if content contains any dx patterns
    pub fn has_patterns(&self, content: &str) -> bool {
        self.patterns.values().any(|regex| regex.is_match(content))
    }

    /// Extract unique component names from matches
    pub fn extract_components(&self, matches: &[PatternMatch]) -> Vec<String> {
        let mut components: Vec<String> =
            matches.iter().map(|m| m.component_name.clone()).collect();

        components.sort();
        components.dedup();
        components
    }
}

impl Default for PatternDetector {
    fn default() -> Self {
        // SAFETY: PatternDetector::new() only fails if regex compilation fails.
        // All regex patterns are compile-time constants that are known to be valid.
        // This is provably always true - the patterns are hardcoded and tested.
        Self::new()
            .expect("PatternDetector regex patterns are compile-time constants and always valid")
    }
}

/// LSP-style position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: usize,
    pub character: usize,
}

/// LSP-style range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// Component injection point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionPoint {
    pub file: PathBuf,
    pub range: Range,
    pub component: String,
    pub tool: DxToolType,
    pub import_needed: bool,
}

/// Analyze file for component injection
pub fn analyze_for_injection(
    path: &Path,
    content: &str,
    matches: &[PatternMatch],
) -> Vec<InjectionPoint> {
    let mut injections = Vec::new();

    // Check if imports already exist
    let has_imports = content.contains("import") || content.contains("require");

    for m in matches {
        injections.push(InjectionPoint {
            file: path.to_path_buf(),
            range: Range {
                start: Position {
                    line: m.line - 1,
                    character: m.column - 1,
                },
                end: Position {
                    line: m.line - 1,
                    character: m.column + m.pattern.len() - 1,
                },
            },
            component: m.component_name.clone(),
            tool: m.tool.clone(),
            import_needed: !has_imports,
        });
    }

    injections
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_detection() {
        let detector = PatternDetector::new().unwrap();
        let content = r#"
            const MyComponent = () => {
                return (
                    <div>
                        <dxButton>Click</dxButton>
                        <dxiHome size={24} />
                        <dxfRoboto>Hello</dxfRoboto>
                    </div>
                );
            };
        "#;

        let matches = detector.detect_in_file(Path::new("test.tsx"), content).unwrap();

        // Note: Regex patterns also match inside tags, so we may get more matches
        assert!(matches.len() >= 3, "Expected at least 3 matches, got {}", matches.len());
        assert!(matches.iter().any(|m| m.tool == DxToolType::Ui));
        assert!(matches.iter().any(|m| m.tool == DxToolType::Icons));
        assert!(matches.iter().any(|m| m.tool == DxToolType::Fonts));
    }

    #[test]
    fn test_component_extraction() {
        let detector = PatternDetector::new().unwrap();
        let content = "dxButton dxButton dxInput dxCard";

        let matches = detector.detect_in_file(Path::new("test.tsx"), content).unwrap();
        let components = detector.extract_components(&matches);

        assert_eq!(components.len(), 3);
        assert!(components.contains(&"Button".to_string()));
        assert!(components.contains(&"Input".to_string()));
        assert!(components.contains(&"Card".to_string()));
    }

    #[test]
    fn test_tool_prefix() {
        assert_eq!(DxToolType::Ui.prefix(), "dx");
        assert_eq!(DxToolType::Icons.prefix(), "dxi");
        assert_eq!(DxToolType::Fonts.prefix(), "dxf");
    }

    #[test]
    fn test_has_patterns() {
        let detector = PatternDetector::new().unwrap();

        assert!(detector.has_patterns("const x = dxButton;"));
        assert!(detector.has_patterns("<dxiHome />"));
        assert!(!detector.has_patterns("const x = regular;"));
    }
}
