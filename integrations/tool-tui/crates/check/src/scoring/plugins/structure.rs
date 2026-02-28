//! Structure and Documentation Scoring Plugin
//!
//! Analyzes file/folder structure, naming conventions, and documentation coverage.

use crate::scoring::plugin::{RuleDefinition, ScoringPlugin};
use crate::scoring_impl::{Category, Severity, Violation};
use std::any::Any;
use std::path::Path;

/// Structure and documentation plugin
pub struct StructurePlugin {
    rules: Vec<RuleDefinition>,
}

impl StructurePlugin {
    /// Create a new structure plugin
    #[must_use]
    pub fn new() -> Self {
        let rules = vec![
            RuleDefinition::new(
                "structure/missing-docs",
                "Missing Documentation",
                Category::StructureAndDocs,
            )
            .with_severity(Severity::Low)
            .with_description("Public API lacks documentation"),
            RuleDefinition::new(
                "structure/file-naming",
                "File Naming Convention",
                Category::StructureAndDocs,
            )
            .with_severity(Severity::Low)
            .with_description("File name doesn't follow conventions"),
            RuleDefinition::new(
                "structure/missing-readme",
                "Missing README",
                Category::StructureAndDocs,
            )
            .with_severity(Severity::Medium)
            .with_description("Directory lacks README file"),
            RuleDefinition::new(
                "structure/missing-license",
                "Missing License",
                Category::StructureAndDocs,
            )
            .with_severity(Severity::Medium)
            .with_description("Project lacks LICENSE file"),
            RuleDefinition::new(
                "structure/todo-fixme",
                "TODO/FIXME Comment",
                Category::StructureAndDocs,
            )
            .with_severity(Severity::Low)
            .with_description("Unresolved TODO or FIXME comment"),
            RuleDefinition::new(
                "structure/commented-code",
                "Commented Out Code",
                Category::StructureAndDocs,
            )
            .with_severity(Severity::Low)
            .with_description("Code is commented out instead of removed"),
            RuleDefinition::new(
                "structure/inconsistent-naming",
                "Inconsistent Naming",
                Category::StructureAndDocs,
            )
            .with_severity(Severity::Low)
            .with_description("Naming style is inconsistent"),
            RuleDefinition::new(
                "structure/missing-type-hints",
                "Missing Type Hints",
                Category::StructureAndDocs,
            )
            .with_severity(Severity::Low)
            .with_description("Function lacks type annotations"),
            RuleDefinition::new("structure/empty-file", "Empty File", Category::StructureAndDocs)
                .with_severity(Severity::Low)
                .with_description("File is empty or nearly empty"),
        ];

        Self { rules }
    }
}

impl Default for StructurePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl ScoringPlugin for StructurePlugin {
    fn id(&self) -> &'static str {
        "structure"
    }

    fn name(&self) -> &'static str {
        "Structure & Documentation"
    }

    fn category(&self) -> Category {
        Category::StructureAndDocs
    }

    fn analyze(&self, path: &Path, content: &[u8], _ast: Option<&dyn Any>) -> Vec<Violation> {
        let mut violations = Vec::new();

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let text = match std::str::from_utf8(content) {
            Ok(t) => t,
            Err(_) => return violations,
        };

        let lines: Vec<&str> = text.lines().collect();

        // Check for empty file
        let non_empty_lines = lines.iter().filter(|l| !l.trim().is_empty()).count();
        if non_empty_lines < 3 && !path.to_string_lossy().contains("__init__") {
            violations.push(Violation {
                category: Category::StructureAndDocs,
                severity: Severity::Low,
                file: path.to_path_buf(),
                line: 1,
                column: 1,
                rule_id: "structure/empty-file".to_string(),
                message: "File is empty or nearly empty".to_string(),
                points: Severity::Low.points(),
            });
        }

        // Check file naming conventions
        if let Some(name) = path.file_stem().and_then(|s| s.to_str())
            && !is_valid_file_name(name, ext)
        {
            violations.push(Violation {
                category: Category::StructureAndDocs,
                severity: Severity::Low,
                file: path.to_path_buf(),
                line: 1,
                column: 1,
                rule_id: "structure/file-naming".to_string(),
                message: format!(
                    "File name '{name}' doesn't follow conventions (use snake_case or kebab-case)"
                ),
                points: Severity::Low.points(),
            });
        }

        // Analyze line by line
        let mut consecutive_comment_lines = 0;
        let mut last_was_code_comment = false;

        for (line_num, line) in lines.iter().enumerate() {
            let line_num = (line_num + 1) as u32;
            let trimmed = line.trim();

            // Track multiline comments (currently unused but reserved for future enhancements)
            let _in_multiline_comment = trimmed.contains("/*") || trimmed.contains("*/");

            // Check for TODO/FIXME
            if trimmed.to_uppercase().contains("TODO") || trimmed.to_uppercase().contains("FIXME") {
                violations.push(Violation {
                    category: Category::StructureAndDocs,
                    severity: Severity::Low,
                    file: path.to_path_buf(),
                    line: line_num,
                    column: 1,
                    rule_id: "structure/todo-fixme".to_string(),
                    message: "Unresolved TODO/FIXME comment".to_string(),
                    points: Severity::Low.points(),
                });
            }

            // Check for commented out code
            let is_comment = is_comment_line(trimmed, ext);
            if is_comment && looks_like_code(trimmed, ext) {
                consecutive_comment_lines += 1;
                last_was_code_comment = true;
            } else {
                if last_was_code_comment && consecutive_comment_lines >= 3 {
                    violations.push(Violation {
                        category: Category::StructureAndDocs,
                        severity: Severity::Low,
                        file: path.to_path_buf(),
                        line: line_num - consecutive_comment_lines,
                        column: 1,
                        rule_id: "structure/commented-code".to_string(),
                        message: format!(
                            "Found {consecutive_comment_lines} lines of commented-out code - remove or use version control"
                        ),
                        points: Severity::Low.points(),
                    });
                }
                consecutive_comment_lines = 0;
                last_was_code_comment = false;
            }

            // Check for missing documentation on public items
            if should_have_docs(trimmed, ext) {
                // Check if previous line(s) have doc comments
                let has_docs = if line_num > 1 {
                    let prev_lines: Vec<&str> =
                        lines[..line_num as usize - 1].iter().rev().take(5).copied().collect();
                    has_doc_comment(&prev_lines, ext)
                } else {
                    false
                };

                if !has_docs {
                    violations.push(Violation {
                        category: Category::StructureAndDocs,
                        severity: Severity::Low,
                        file: path.to_path_buf(),
                        line: line_num,
                        column: 1,
                        rule_id: "structure/missing-docs".to_string(),
                        message: "Public item lacks documentation".to_string(),
                        points: Severity::Low.points(),
                    });
                }
            }

            // Check for missing type hints (Python)
            if ext == "py" && is_function_def(trimmed) && !has_type_hints(trimmed) {
                violations.push(Violation {
                    category: Category::StructureAndDocs,
                    severity: Severity::Low,
                    file: path.to_path_buf(),
                    line: line_num,
                    column: 1,
                    rule_id: "structure/missing-type-hints".to_string(),
                    message: "Function lacks type annotations".to_string(),
                    points: Severity::Low.points(),
                });
            }
        }

        violations
    }

    fn rules(&self) -> &[RuleDefinition] {
        &self.rules
    }

    fn description(&self) -> &'static str {
        "Analyzes code structure, naming conventions, and documentation coverage"
    }
}

/// Check if file name follows conventions
fn is_valid_file_name(name: &str, ext: &str) -> bool {
    // Allow special files
    if name.starts_with('.') || name == "README" || name == "LICENSE" || name == "Makefile" {
        return true;
    }

    // Check for common conventions based on language
    match ext {
        // snake_case for Rust, Python
        "rs" | "py" => name.chars().all(|c| c.is_lowercase() || c == '_' || c.is_numeric()),
        // kebab-case or PascalCase for JS/TS (components)
        "js" | "ts" | "jsx" | "tsx" => {
            let is_kebab = name.chars().all(|c| c.is_lowercase() || c == '-' || c.is_numeric());
            let is_pascal = name.chars().next().is_some_and(char::is_uppercase)
                && name.chars().all(char::is_alphanumeric);
            let is_snake = name.chars().all(|c| c.is_lowercase() || c == '_' || c.is_numeric());
            is_kebab || is_pascal || is_snake
        }
        // PascalCase for Go, C#, Java
        "go" | "cs" | "java" => {
            let is_snake = name.chars().all(|c| c.is_lowercase() || c == '_' || c.is_numeric());
            let is_pascal = name.chars().all(char::is_alphanumeric);
            is_snake || is_pascal
        }
        _ => true,
    }
}

/// Check if line is a comment
fn is_comment_line(line: &str, ext: &str) -> bool {
    match ext {
        "py" | "rb" | "sh" | "yaml" | "yml" | "toml" => line.starts_with('#'),
        "rs" | "js" | "ts" | "jsx" | "tsx" | "go" | "c" | "cpp" | "java" | "cs" | "kt" => {
            line.starts_with("//") || line.starts_with("/*") || line.starts_with('*')
        }
        _ => line.starts_with("//") || line.starts_with('#'),
    }
}

/// Check if commented line looks like code
fn looks_like_code(line: &str, ext: &str) -> bool {
    let without_comment = match ext {
        "py" | "rb" | "sh" => line.trim_start_matches('#').trim(),
        _ => line
            .trim_start_matches("//")
            .trim_start_matches("/*")
            .trim_start_matches('*')
            .trim(),
    };

    // Code indicators
    let code_patterns = [
        "=", ";", "(", ")", "{", "}", "[", "]", "if ", "else", "for ", "while ", "return ", "let ",
        "const ", "var ", "fn ", "def ", "class ", "import ", "from ", "use ", "require",
    ];

    code_patterns.iter().any(|p| without_comment.contains(p))
}

/// Check if item should have documentation
fn should_have_docs(line: &str, ext: &str) -> bool {
    match ext {
        "rs" => {
            line.starts_with("pub fn ")
                || line.starts_with("pub struct ")
                || line.starts_with("pub enum ")
                || line.starts_with("pub trait ")
        }
        "py" => (line.starts_with("def ") || line.starts_with("class ")) && !line.contains('_'),
        "js" | "ts" | "jsx" | "tsx" => {
            line.contains("export ")
                && (line.contains("function") || line.contains("class") || line.contains("const"))
        }
        "go" => line.starts_with("func ") && line.chars().nth(5).is_some_and(char::is_uppercase),
        _ => false,
    }
}

/// Check if previous lines have doc comments
fn has_doc_comment(prev_lines: &[&str], ext: &str) -> bool {
    for line in prev_lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match ext {
            "rs" => {
                if trimmed.starts_with("///") || trimmed.starts_with("//!") {
                    return true;
                }
            }
            "py" => {
                if trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''") {
                    return true;
                }
            }
            "js" | "ts" | "jsx" | "tsx" => {
                if trimmed.starts_with("/**") || trimmed.starts_with("* ") {
                    return true;
                }
            }
            "go" => {
                if trimmed.starts_with("//") {
                    return true;
                }
            }
            _ => {}
        }

        // Stop if we hit non-comment, non-empty line
        if !is_comment_line(trimmed, ext) {
            break;
        }
    }
    false
}

/// Check if line is a function definition (Python)
fn is_function_def(line: &str) -> bool {
    line.starts_with("def ") || line.starts_with("async def ")
}

/// Check if Python function has type hints
fn has_type_hints(line: &str) -> bool {
    // Check for return type annotation
    if line.contains("->") {
        return true;
    }
    // Check for parameter type annotations
    if let Some(start) = line.find('(')
        && let Some(end) = line.find(')')
    {
        let params = &line[start + 1..end];
        if params.trim().is_empty() || params.contains(':') {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_structure_plugin_creation() {
        let plugin = StructurePlugin::new();
        assert_eq!(plugin.id(), "structure");
        assert_eq!(plugin.category(), Category::StructureAndDocs);
    }

    #[test]
    fn test_todo_detection() {
        let plugin = StructurePlugin::new();
        let content = b"// TODO: fix this later";
        let path = Path::new("test.rs");

        let violations = plugin.analyze(path, content, None);
        assert!(violations.iter().any(|v| v.rule_id == "structure/todo-fixme"));
    }

    #[test]
    fn test_file_naming() {
        assert!(is_valid_file_name("my_module", "rs"));
        assert!(is_valid_file_name("my_module", "py"));
        assert!(is_valid_file_name("MyComponent", "tsx"));
        assert!(!is_valid_file_name("my Module", "rs"));
    }

    #[test]
    fn test_empty_file() {
        let plugin = StructurePlugin::new();
        let content = b"\n\n";
        let path = Path::new("test.rs");

        let violations = plugin.analyze(path, content, None);
        assert!(violations.iter().any(|v| v.rule_id == "structure/empty-file"));
    }
}
