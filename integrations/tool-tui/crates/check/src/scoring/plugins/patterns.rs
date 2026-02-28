//! Design Patterns Scoring Plugin
//!
//! Detects code smells, complexity issues, and design pattern violations.

use crate::scoring::plugin::{RuleDefinition, ScoringPlugin};
use crate::scoring_impl::{Category, Severity, Violation};
use std::any::Any;
use std::path::Path;

/// Design patterns plugin for code quality analysis
pub struct PatternsPlugin {
    rules: Vec<RuleDefinition>,
    max_function_lines: u32,
    max_file_lines: u32,
    max_nesting_depth: u32,
    max_parameters: u32,
    max_complexity: u32,
}

impl PatternsPlugin {
    /// Create a new patterns plugin with default thresholds
    #[must_use]
    pub fn new() -> Self {
        let rules = vec![
            RuleDefinition::new(
                "patterns/long-function",
                "Long Function",
                Category::DesignPatterns,
            )
            .with_severity(Severity::Medium)
            .with_description("Function exceeds recommended line count"),
            RuleDefinition::new("patterns/large-file", "Large File", Category::DesignPatterns)
                .with_severity(Severity::Medium)
                .with_description("File exceeds recommended line count"),
            RuleDefinition::new("patterns/deep-nesting", "Deep Nesting", Category::DesignPatterns)
                .with_severity(Severity::High)
                .with_description("Code has excessive nesting depth"),
            RuleDefinition::new(
                "patterns/too-many-params",
                "Too Many Parameters",
                Category::DesignPatterns,
            )
            .with_severity(Severity::Medium)
            .with_description("Function has too many parameters"),
            RuleDefinition::new("patterns/magic-number", "Magic Number", Category::DesignPatterns)
                .with_severity(Severity::Low)
                .with_description("Unexplained numeric literal in code"),
            RuleDefinition::new("patterns/god-class", "God Class", Category::DesignPatterns)
                .with_severity(Severity::High)
                .with_description("Class has too many responsibilities"),
            RuleDefinition::new(
                "patterns/duplicate-code",
                "Duplicate Code",
                Category::DesignPatterns,
            )
            .with_severity(Severity::Medium)
            .with_description("Duplicated code block detected"),
            RuleDefinition::new("patterns/dead-code", "Dead Code", Category::DesignPatterns)
                .with_severity(Severity::Low)
                .with_description("Unreachable or unused code detected"),
            RuleDefinition::new(
                "patterns/complex-condition",
                "Complex Condition",
                Category::DesignPatterns,
            )
            .with_severity(Severity::Medium)
            .with_description("Condition is too complex"),
            RuleDefinition::new(
                "patterns/callback-hell",
                "Callback Hell",
                Category::DesignPatterns,
            )
            .with_severity(Severity::Medium)
            .with_description("Excessive callback nesting"),
        ];

        Self {
            rules,
            max_function_lines: 50,
            max_file_lines: 500,
            max_nesting_depth: 4,
            max_parameters: 5,
            max_complexity: 10,
        }
    }

    /// Create with custom thresholds
    #[must_use]
    pub fn with_thresholds(
        mut self,
        max_function_lines: u32,
        max_file_lines: u32,
        max_nesting_depth: u32,
        max_parameters: u32,
    ) -> Self {
        self.max_function_lines = max_function_lines;
        self.max_file_lines = max_file_lines;
        self.max_nesting_depth = max_nesting_depth;
        self.max_parameters = max_parameters;
        self
    }
}

impl Default for PatternsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl ScoringPlugin for PatternsPlugin {
    fn id(&self) -> &'static str {
        "patterns"
    }

    fn name(&self) -> &'static str {
        "Design Patterns Analysis"
    }

    fn category(&self) -> Category {
        Category::DesignPatterns
    }

    fn analyze(&self, path: &Path, content: &[u8], _ast: Option<&dyn Any>) -> Vec<Violation> {
        let mut violations = Vec::new();

        let text = match std::str::from_utf8(content) {
            Ok(t) => t,
            Err(_) => return violations,
        };

        let lines: Vec<&str> = text.lines().collect();
        let total_lines = lines.len() as u32;

        // Check file size
        if total_lines > self.max_file_lines {
            violations.push(Violation {
                category: Category::DesignPatterns,
                severity: Severity::Medium,
                file: path.to_path_buf(),
                line: 1,
                column: 1,
                rule_id: "patterns/large-file".to_string(),
                message: format!(
                    "File has {} lines (max recommended: {})",
                    total_lines, self.max_file_lines
                ),
                points: Severity::Medium.points(),
            });
        }

        // Track function boundaries and nesting
        let mut current_nesting = 0u32;
        let mut max_nesting_line = 0u32;
        let mut max_nesting_found = 0u32;
        let mut function_start: Option<(u32, String)> = None;

        for (line_num, line) in lines.iter().enumerate() {
            let line_num = (line_num + 1) as u32;
            let trimmed = line.trim();

            // Track nesting depth
            let opens = line.matches('{').count() as u32 + line.matches('(').count() as u32;
            let closes = line.matches('}').count() as u32 + line.matches(')').count() as u32;

            if opens > closes {
                current_nesting += opens - closes;
                if current_nesting > max_nesting_found {
                    max_nesting_found = current_nesting;
                    max_nesting_line = line_num;
                }
            } else if closes > opens {
                current_nesting = current_nesting.saturating_sub(closes - opens);
            }

            // Detect function start
            if is_function_start(trimmed) {
                if let Some((start_line, name)) = function_start.take() {
                    let func_lines = line_num - start_line;
                    if func_lines > self.max_function_lines {
                        violations.push(Violation {
                            category: Category::DesignPatterns,
                            severity: Severity::Medium,
                            file: path.to_path_buf(),
                            line: start_line,
                            column: 1,
                            rule_id: "patterns/long-function".to_string(),
                            message: format!(
                                "Function '{}' has {} lines (max: {})",
                                name, func_lines, self.max_function_lines
                            ),
                            points: Severity::Medium.points(),
                        });
                    }
                }
                function_start = Some((line_num, extract_function_name(trimmed)));
            }

            // Check for too many parameters
            if let Some(param_count) = count_parameters(trimmed)
                && param_count > self.max_parameters
            {
                violations.push(Violation {
                    category: Category::DesignPatterns,
                    severity: Severity::Medium,
                    file: path.to_path_buf(),
                    line: line_num,
                    column: 1,
                    rule_id: "patterns/too-many-params".to_string(),
                    message: format!(
                        "Function has {} parameters (max: {})",
                        param_count, self.max_parameters
                    ),
                    points: Severity::Medium.points(),
                });
            }

            // Check for magic numbers
            if has_magic_number(trimmed) {
                violations.push(Violation {
                    category: Category::DesignPatterns,
                    severity: Severity::Low,
                    file: path.to_path_buf(),
                    line: line_num,
                    column: 1,
                    rule_id: "patterns/magic-number".to_string(),
                    message: "Consider extracting magic number to a named constant".to_string(),
                    points: Severity::Low.points(),
                });
            }

            // Check for complex conditions
            if has_complex_condition(trimmed) {
                violations.push(Violation {
                    category: Category::DesignPatterns,
                    severity: Severity::Medium,
                    file: path.to_path_buf(),
                    line: line_num,
                    column: 1,
                    rule_id: "patterns/complex-condition".to_string(),
                    message:
                        "Condition is too complex - consider extracting to a variable or function"
                            .to_string(),
                    points: Severity::Medium.points(),
                });
            }
        }

        // Check max nesting
        if max_nesting_found > self.max_nesting_depth {
            violations.push(Violation {
                category: Category::DesignPatterns,
                severity: Severity::High,
                file: path.to_path_buf(),
                line: max_nesting_line,
                column: 1,
                rule_id: "patterns/deep-nesting".to_string(),
                message: format!(
                    "Code nesting depth {} exceeds max {} - consider refactoring",
                    max_nesting_found, self.max_nesting_depth
                ),
                points: Severity::High.points(),
            });
        }

        violations
    }

    fn rules(&self) -> &[RuleDefinition] {
        &self.rules
    }

    fn description(&self) -> &'static str {
        "Analyzes code for design pattern violations, code smells, and complexity issues"
    }
}

/// Check if line starts a function definition
fn is_function_start(line: &str) -> bool {
    // JavaScript/TypeScript
    if line.contains("function ") || line.contains("=> {") || line.contains("async ") {
        return true;
    }
    // Python
    if line.starts_with("def ") || line.starts_with("async def ") {
        return true;
    }
    // Rust
    if line.starts_with("fn ") || line.starts_with("pub fn ") || line.starts_with("async fn ") {
        return true;
    }
    // Go
    if line.starts_with("func ") {
        return true;
    }
    false
}

/// Extract function name from line
fn extract_function_name(line: &str) -> String {
    // Try to extract function name
    let patterns = [
        (r"function\s+(\w+)", 1),
        (r"def\s+(\w+)", 1),
        (r"fn\s+(\w+)", 1),
        (r"func\s+(\w+)", 1),
        (r"(\w+)\s*=\s*(?:async\s*)?\(", 1),
    ];

    for (pattern, group) in patterns {
        if let Ok(re) = regex::Regex::new(pattern)
            && let Some(caps) = re.captures(line)
            && let Some(m) = caps.get(group)
        {
            return m.as_str().to_string();
        }
    }

    "anonymous".to_string()
}

/// Count function parameters
fn count_parameters(line: &str) -> Option<u32> {
    if !line.contains('(') {
        return None;
    }

    // Find parameter list
    if let Some(start) = line.find('(')
        && let Some(end) = line[start..].find(')')
    {
        let params = &line[start + 1..start + end];
        if params.trim().is_empty() {
            return Some(0);
        }
        // Count commas + 1
        return Some(params.matches(',').count() as u32 + 1);
    }

    None
}

/// Check for magic numbers (unexplained numeric literals)
fn has_magic_number(line: &str) -> bool {
    // Skip common cases
    if line.contains("const ") || line.contains("let ") || line.contains("var ") {
        return false;
    }
    if line.trim().starts_with("//") || line.trim().starts_with('#') {
        return false;
    }

    // Look for numbers that aren't 0, 1, 2, or common values
    let re = regex::Regex::new(r"\b([3-9]|[1-9]\d{2,})\b").unwrap();
    if re.is_match(line) {
        // Exclude array indices, loop counters, etc.
        if line.contains('[') || line.contains("for") || line.contains("range") {
            return false;
        }
        return true;
    }

    false
}

/// Check for complex conditions (too many logical operators)
fn has_complex_condition(line: &str) -> bool {
    if !line.contains("if") && !line.contains("while") && !line.contains('?') {
        return false;
    }

    let logical_ops = line.matches("&&").count()
        + line.matches("||").count()
        + line.matches(" and ").count()
        + line.matches(" or ").count();

    logical_ops >= 3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patterns_plugin_creation() {
        let plugin = PatternsPlugin::new();
        assert_eq!(plugin.id(), "patterns");
        assert_eq!(plugin.category(), Category::DesignPatterns);
    }

    #[test]
    fn test_large_file_detection() {
        let plugin = PatternsPlugin::new().with_thresholds(50, 10, 4, 5);
        let content = "line\n".repeat(20);
        let path = Path::new("test.rs");

        let violations = plugin.analyze(path, content.as_bytes(), None);
        assert!(violations.iter().any(|v| v.rule_id == "patterns/large-file"));
    }

    #[test]
    fn test_deep_nesting_detection() {
        let plugin = PatternsPlugin::new();
        let content = r#"
            fn test() {
                if true {
                    if true {
                        if true {
                            if true {
                                if true {
                                    println!("deep");
                                }
                            }
                        }
                    }
                }
            }
        "#;
        let path = Path::new("test.rs");

        let violations = plugin.analyze(path, content.as_bytes(), None);
        assert!(violations.iter().any(|v| v.rule_id == "patterns/deep-nesting"));
    }

    #[test]
    fn test_too_many_params() {
        let plugin = PatternsPlugin::new();
        let content = "fn test(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) {}";
        let path = Path::new("test.rs");

        let violations = plugin.analyze(path, content.as_bytes(), None);
        assert!(violations.iter().any(|v| v.rule_id == "patterns/too-many-params"));
    }
}
