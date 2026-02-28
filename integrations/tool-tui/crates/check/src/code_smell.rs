//! Code Smell Detection
//!
//! Detects common code smells that indicate poor design:
//! - Long methods/functions (>50 lines)
//! - Large classes (>500 lines)
//! - Too many parameters (>5)
//! - Deep nesting (>4 levels)
//! - Duplicate code blocks
//! - Dead code (unused functions, variables)
//! - Magic numbers and strings
//!
//! All violations are mapped to the `DesignPatterns` category.

use crate::complexity::ComplexityCalculator;
use crate::diagnostics::{Diagnostic, DiagnosticSeverity, Span};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Code smell types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodeSmellType {
    LongMethod,
    LargeClass,
    TooManyParameters,
    DeepNesting,
    DuplicateCode,
    DeadCode,
    MagicNumber,
    MagicString,
    HighCyclomaticComplexity,
    HighCognitiveComplexity,
}

impl CodeSmellType {
    #[must_use]
    pub fn rule_id(&self) -> &'static str {
        match self {
            CodeSmellType::LongMethod => "long-method",
            CodeSmellType::LargeClass => "large-class",
            CodeSmellType::TooManyParameters => "too-many-parameters",
            CodeSmellType::DeepNesting => "deep-nesting",
            CodeSmellType::DuplicateCode => "duplicate-code",
            CodeSmellType::DeadCode => "dead-code",
            CodeSmellType::MagicNumber => "magic-number",
            CodeSmellType::MagicString => "magic-string",
            CodeSmellType::HighCyclomaticComplexity => "high-cyclomatic-complexity",
            CodeSmellType::HighCognitiveComplexity => "high-cognitive-complexity",
        }
    }

    #[must_use]
    pub fn message(&self, details: &str) -> String {
        match self {
            CodeSmellType::LongMethod => format!("Method/function is too long: {details}"),
            CodeSmellType::LargeClass => format!("Class is too large: {details}"),
            CodeSmellType::TooManyParameters => format!("Too many parameters: {details}"),
            CodeSmellType::DeepNesting => format!("Code is nested too deeply: {details}"),
            CodeSmellType::DuplicateCode => format!("Duplicate code detected: {details}"),
            CodeSmellType::DeadCode => format!("Dead code detected: {details}"),
            CodeSmellType::MagicNumber => format!("Magic number detected: {details}"),
            CodeSmellType::MagicString => format!("Magic string detected: {details}"),
            CodeSmellType::HighCyclomaticComplexity => {
                format!("High cyclomatic complexity: {details}")
            }
            CodeSmellType::HighCognitiveComplexity => {
                format!("High cognitive complexity: {details}")
            }
        }
    }

    #[must_use]
    pub fn severity(&self) -> DiagnosticSeverity {
        match self {
            CodeSmellType::LongMethod => DiagnosticSeverity::Warning,
            CodeSmellType::LargeClass => DiagnosticSeverity::Warning,
            CodeSmellType::TooManyParameters => DiagnosticSeverity::Warning,
            CodeSmellType::DeepNesting => DiagnosticSeverity::Warning,
            CodeSmellType::DuplicateCode => DiagnosticSeverity::Info,
            CodeSmellType::DeadCode => DiagnosticSeverity::Info,
            CodeSmellType::MagicNumber => DiagnosticSeverity::Info,
            CodeSmellType::MagicString => DiagnosticSeverity::Info,
            CodeSmellType::HighCyclomaticComplexity => DiagnosticSeverity::Warning,
            CodeSmellType::HighCognitiveComplexity => DiagnosticSeverity::Warning,
        }
    }
}

/// Configuration for code smell detection thresholds
#[derive(Debug, Clone)]
pub struct CodeSmellConfig {
    pub max_function_lines: usize,
    pub max_class_lines: usize,
    pub max_parameters: usize,
    pub max_nesting_depth: usize,
    pub min_duplicate_lines: usize,
    pub detect_magic_numbers: bool,
    pub detect_magic_strings: bool,
    pub magic_number_exceptions: HashSet<String>,
    pub max_cyclomatic_complexity: usize,
    pub max_cognitive_complexity: usize,
}

impl Default for CodeSmellConfig {
    fn default() -> Self {
        let mut magic_number_exceptions = HashSet::new();
        // Common acceptable numbers
        magic_number_exceptions.insert("0".to_string());
        magic_number_exceptions.insert("1".to_string());
        magic_number_exceptions.insert("-1".to_string());
        magic_number_exceptions.insert("2".to_string());

        Self {
            max_function_lines: 50,
            max_class_lines: 500,
            max_parameters: 5,
            max_nesting_depth: 4,
            min_duplicate_lines: 6,
            detect_magic_numbers: true,
            detect_magic_strings: true,
            magic_number_exceptions,
            max_cyclomatic_complexity: 10,
            max_cognitive_complexity: 15,
        }
    }
}

/// Code smell detector
pub struct CodeSmellDetector {
    config: CodeSmellConfig,
    complexity_calc: ComplexityCalculator,
}

impl CodeSmellDetector {
    #[must_use]
    pub fn new(config: CodeSmellConfig) -> Self {
        let complexity_calc = ComplexityCalculator::new(
            config.max_cyclomatic_complexity,
            config.max_cognitive_complexity,
        );
        Self {
            config,
            complexity_calc,
        }
    }

    #[must_use]
    pub fn with_default_config() -> Self {
        Self::new(CodeSmellConfig::default())
    }

    /// Detect all code smells in a file
    #[must_use]
    pub fn detect(&self, path: &Path, source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Detect long methods/functions
        diagnostics.extend(self.detect_long_methods(path, source));

        // Detect large classes
        diagnostics.extend(self.detect_large_classes(path, source));

        // Detect too many parameters
        diagnostics.extend(self.detect_too_many_parameters(path, source));

        // Detect deep nesting
        diagnostics.extend(self.detect_deep_nesting(path, source));

        // Detect duplicate code
        diagnostics.extend(self.detect_duplicate_code(path, source));

        // Detect dead code
        diagnostics.extend(self.detect_dead_code(path, source));

        // Detect magic numbers
        if self.config.detect_magic_numbers {
            diagnostics.extend(self.detect_magic_numbers(path, source));
        }

        // Detect magic strings
        if self.config.detect_magic_strings {
            diagnostics.extend(self.detect_magic_strings(path, source));
        }

        // Detect complexity issues
        diagnostics.extend(self.detect_complexity_issues(path, source));

        diagnostics
    }

    /// Detect functions with high complexity metrics
    fn detect_complexity_issues(&self, path: &Path, source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let metrics = self.complexity_calc.calculate_all(source);

        for metric in metrics {
            // Check cyclomatic complexity
            if metric.cyclomatic > self.config.max_cyclomatic_complexity {
                diagnostics.push(Diagnostic {
                    severity: CodeSmellType::HighCyclomaticComplexity.severity(),
                    file: path.to_path_buf(),
                    span: Span {
                        start: metric.start_line as u32 + 1,
                        end: metric.end_line as u32 + 1,
                    },
                    rule_id: CodeSmellType::HighCyclomaticComplexity.rule_id().to_string(),
                    message: CodeSmellType::HighCyclomaticComplexity.message(&format!(
                        "function '{}' has cyclomatic complexity {} (max {}), LOC: {}, SLOC: {}",
                        metric.name,
                        metric.cyclomatic,
                        self.config.max_cyclomatic_complexity,
                        metric.loc,
                        metric.sloc
                    )),
                    suggestion: Some("Break down complex logic into smaller functions".to_string()),
                    related: Vec::new(),
                    fix: None,
                });
            }

            // Check cognitive complexity
            if metric.cognitive > self.config.max_cognitive_complexity {
                diagnostics.push(Diagnostic {
                    severity: CodeSmellType::HighCognitiveComplexity.severity(),
                    file: path.to_path_buf(),
                    span: Span {
                        start: metric.start_line as u32 + 1,
                        end: metric.end_line as u32 + 1,
                    },
                    rule_id: CodeSmellType::HighCognitiveComplexity.rule_id().to_string(),
                    message: CodeSmellType::HighCognitiveComplexity.message(&format!(
                        "function '{}' has cognitive complexity {} (max {}), LOC: {}, SLOC: {}",
                        metric.name,
                        metric.cognitive,
                        self.config.max_cognitive_complexity,
                        metric.loc,
                        metric.sloc
                    )),
                    suggestion: Some("Reduce nesting and simplify control flow".to_string()),
                    related: Vec::new(),
                    fix: None,
                });
            }
        }

        diagnostics
    }

    /// Detect long methods/functions (>50 lines)
    fn detect_long_methods(&self, path: &Path, source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        // Simple pattern matching for function/method declarations
        let patterns = [
            r"fn\s+\w+",                // Rust
            r"function\s+\w+",          // JavaScript
            r"def\s+\w+",               // Python
            r"func\s+\w+",              // Go
            r"void\s+\w+\s*\(",         // C/C++
            r"public\s+\w+\s+\w+\s*\(", // Java/C#
        ];

        for (line_num, line) in lines.iter().enumerate() {
            for pattern in &patterns {
                if regex::Regex::new(pattern).ok().and_then(|re| re.find(line)).is_some() {
                    // Found a function declaration, count lines until closing brace
                    let function_lines = self.count_function_lines(&lines, line_num);

                    if function_lines > self.config.max_function_lines {
                        diagnostics.push(Diagnostic {
                            severity: CodeSmellType::LongMethod.severity(),
                            file: path.to_path_buf(),
                            span: Span {
                                start: line_num as u32 + 1,
                                end: (line_num + function_lines) as u32 + 1,
                            },
                            rule_id: CodeSmellType::LongMethod.rule_id().to_string(),
                            message: CodeSmellType::LongMethod.message(&format!(
                                "{} lines (max {})",
                                function_lines, self.config.max_function_lines
                            )),
                            suggestion: Some(
                                "Break this function into smaller functions".to_string(),
                            ),
                            related: Vec::new(),
                            fix: None,
                        });
                    }
                }
            }
        }

        diagnostics
    }

    /// Count lines in a function starting from the declaration line
    fn count_function_lines(&self, lines: &[&str], start_line: usize) -> usize {
        let mut brace_count = 0;
        let mut line_count = 0;
        let mut started = false;

        for line in lines.iter().skip(start_line) {
            line_count += 1;

            for ch in line.chars() {
                if ch == '{' {
                    brace_count += 1;
                    started = true;
                } else if ch == '}' {
                    brace_count -= 1;
                    if started && brace_count == 0 {
                        return line_count;
                    }
                }
            }
        }

        line_count
    }

    /// Detect large classes (>500 lines)
    fn detect_large_classes(&self, path: &Path, source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        // Patterns for class declarations
        let patterns = [
            r"class\s+\w+",          // JavaScript/TypeScript
            r"struct\s+\w+",         // Rust
            r"type\s+\w+\s+struct",  // Go
            r"public\s+class\s+\w+", // Java/C#
        ];

        for (line_num, line) in lines.iter().enumerate() {
            for pattern in &patterns {
                if regex::Regex::new(pattern).ok().and_then(|re| re.find(line)).is_some() {
                    let class_lines = self.count_function_lines(&lines, line_num);

                    if class_lines > self.config.max_class_lines {
                        diagnostics.push(Diagnostic {
                            severity: CodeSmellType::LargeClass.severity(),
                            file: path.to_path_buf(),
                            span: Span {
                                start: line_num as u32 + 1,
                                end: (line_num + class_lines) as u32 + 1,
                            },
                            rule_id: CodeSmellType::LargeClass.rule_id().to_string(),
                            message: CodeSmellType::LargeClass.message(&format!(
                                "{} lines (max {})",
                                class_lines, self.config.max_class_lines
                            )),
                            suggestion: Some("Break this class into smaller classes".to_string()),
                            related: Vec::new(),
                            fix: None,
                        });
                    }
                }
            }
        }

        diagnostics
    }

    /// Detect functions with too many parameters (>5)
    fn detect_too_many_parameters(&self, path: &Path, source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        for (line_num, line) in lines.iter().enumerate() {
            // Count parameters in function declarations
            if let Some(params_str) = self.extract_parameters(line) {
                let param_count = self.count_parameters(&params_str);

                if param_count > self.config.max_parameters {
                    diagnostics.push(Diagnostic {
                        severity: CodeSmellType::TooManyParameters.severity(),
                        file: path.to_path_buf(),
                        span: Span {
                            start: line_num as u32 + 1,
                            end: line_num as u32 + 1,
                        },
                        rule_id: CodeSmellType::TooManyParameters.rule_id().to_string(),
                        message: CodeSmellType::TooManyParameters.message(&format!(
                            "{} parameters (max {})",
                            param_count, self.config.max_parameters
                        )),
                        suggestion: Some("Use object parameter or refactor function".to_string()),
                        related: Vec::new(),
                        fix: None,
                    });
                }
            }
        }

        diagnostics
    }

    /// Extract parameter list from function declaration
    fn extract_parameters(&self, line: &str) -> Option<String> {
        if let Some(start) = line.find('(')
            && let Some(end) = line[start..].find(')')
        {
            return Some(line[start + 1..start + end].to_string());
        }
        None
    }

    /// Count parameters in parameter list
    fn count_parameters(&self, params: &str) -> usize {
        if params.trim().is_empty() {
            return 0;
        }

        // Simple comma counting (doesn't handle nested generics perfectly)
        params.split(',').filter(|p| !p.trim().is_empty()).count()
    }

    /// Detect deep nesting (>4 levels)
    fn detect_deep_nesting(&self, path: &Path, source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        for (line_num, _line) in lines.iter().enumerate() {
            let nesting_level = self.calculate_nesting_level(&lines, line_num);

            if nesting_level > self.config.max_nesting_depth {
                diagnostics.push(Diagnostic {
                    severity: CodeSmellType::DeepNesting.severity(),
                    file: path.to_path_buf(),
                    span: Span {
                        start: line_num as u32 + 1,
                        end: line_num as u32 + 1,
                    },
                    rule_id: CodeSmellType::DeepNesting.rule_id().to_string(),
                    message: CodeSmellType::DeepNesting.message(&format!(
                        "{} levels (max {})",
                        nesting_level, self.config.max_nesting_depth
                    )),
                    suggestion: Some("Extract nested logic into separate functions".to_string()),
                    related: Vec::new(),
                    fix: None,
                });
            }
        }

        diagnostics
    }

    /// Calculate nesting level at a specific line
    fn calculate_nesting_level(&self, lines: &[&str], target_line: usize) -> usize {
        let mut level: usize = 0;

        for line in lines.iter().take(target_line + 1) {
            for ch in line.chars() {
                if ch == '{' {
                    level += 1;
                } else if ch == '}' {
                    level = level.saturating_sub(1);
                }
            }
        }

        level
    }

    /// Detect duplicate code blocks
    fn detect_duplicate_code(&self, path: &Path, source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        // Simple duplicate detection: hash consecutive line blocks
        let mut block_hashes: HashMap<String, Vec<usize>> = HashMap::new();

        for i in 0..lines.len().saturating_sub(self.config.min_duplicate_lines) {
            let block: String = lines[i..i + self.config.min_duplicate_lines]
                .iter()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty() && !l.starts_with("//") && !l.starts_with('#'))
                .collect::<Vec<_>>()
                .join("\n");

            if !block.is_empty() {
                block_hashes.entry(block).or_default().push(i);
            }
        }

        // Report duplicates
        for locations in block_hashes.values() {
            if locations.len() > 1 {
                for &line_num in locations {
                    diagnostics.push(Diagnostic {
                        severity: CodeSmellType::DuplicateCode.severity(),
                        file: path.to_path_buf(),
                        span: Span {
                            start: line_num as u32 + 1,
                            end: (line_num + self.config.min_duplicate_lines) as u32 + 1,
                        },
                        rule_id: CodeSmellType::DuplicateCode.rule_id().to_string(),
                        message: CodeSmellType::DuplicateCode
                            .message(&format!("found {} times", locations.len())),
                        suggestion: Some("Extract to shared function".to_string()),
                        related: Vec::new(),
                        fix: None,
                    });
                }
            }
        }

        diagnostics
    }

    /// Detect dead code (unused functions, variables)
    fn detect_dead_code(&self, path: &Path, source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        // Collect all function/variable definitions
        let mut definitions: HashMap<String, usize> = HashMap::new();
        let mut usages: HashSet<String> = HashSet::new();

        // Simple pattern matching for definitions
        for (line_num, line) in lines.iter().enumerate() {
            // Function definitions
            if let Some(name) = self.extract_function_name(line) {
                definitions.insert(name, line_num);
            }

            // Variable definitions (let, const, var)
            if let Some(name) = self.extract_variable_name(line) {
                definitions.insert(name, line_num);
            }
        }

        // Collect usages
        for line in &lines {
            for name in definitions.keys() {
                if line.contains(name) {
                    usages.insert(name.clone());
                }
            }
        }

        // Report unused definitions
        for (name, line_num) in &definitions {
            let usage_count = lines.iter().filter(|l| l.contains(name)).count();

            // If only appears once (the definition itself), it's unused
            if usage_count <= 1 && !name.starts_with('_') {
                diagnostics.push(Diagnostic {
                    severity: CodeSmellType::DeadCode.severity(),
                    file: path.to_path_buf(),
                    span: Span {
                        start: *line_num as u32 + 1,
                        end: *line_num as u32 + 1,
                    },
                    rule_id: CodeSmellType::DeadCode.rule_id().to_string(),
                    message: CodeSmellType::DeadCode.message(&format!("'{name}' is never used")),
                    suggestion: Some("Remove unused code".to_string()),
                    related: Vec::new(),
                    fix: None,
                });
            }
        }

        diagnostics
    }

    /// Extract function name from declaration
    fn extract_function_name(&self, line: &str) -> Option<String> {
        let patterns = [
            (r"fn\s+(\w+)", 1),
            (r"function\s+(\w+)", 1),
            (r"def\s+(\w+)", 1),
            (r"func\s+(\w+)", 1),
        ];

        for (pattern, group) in &patterns {
            if let Ok(re) = regex::Regex::new(pattern)
                && let Some(caps) = re.captures(line)
                && let Some(name) = caps.get(*group)
            {
                return Some(name.as_str().to_string());
            }
        }

        None
    }

    /// Extract variable name from declaration
    fn extract_variable_name(&self, line: &str) -> Option<String> {
        let patterns = [r"let\s+(\w+)", r"const\s+(\w+)", r"var\s+(\w+)"];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern)
                && let Some(caps) = re.captures(line)
                && let Some(name) = caps.get(1)
            {
                return Some(name.as_str().to_string());
            }
        }

        None
    }

    /// Detect magic numbers
    fn detect_magic_numbers(&self, path: &Path, source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        // Pattern for numeric literals in code
        let number_pattern = match regex::Regex::new(r"\b(\d+\.?\d*)\b") {
            Ok(re) => re,
            Err(_) => return diagnostics,
        };

        for (line_num, line) in lines.iter().enumerate() {
            // Skip comments and strings
            if line.trim().starts_with("//") || line.trim().starts_with('#') {
                continue;
            }

            for cap in number_pattern.captures_iter(line) {
                if let Some(number) = cap.get(1) {
                    let num_str = number.as_str();

                    // Skip if in exception list
                    if self.config.magic_number_exceptions.contains(num_str) {
                        continue;
                    }

                    // Skip if it looks like a constant definition
                    if line.to_uppercase().contains("CONST") || line.contains("final") {
                        continue;
                    }

                    diagnostics.push(Diagnostic {
                        severity: CodeSmellType::MagicNumber.severity(),
                        file: path.to_path_buf(),
                        span: Span {
                            start: line_num as u32 + 1,
                            end: line_num as u32 + 1,
                        },
                        rule_id: CodeSmellType::MagicNumber.rule_id().to_string(),
                        message: CodeSmellType::MagicNumber.message(&format!("'{num_str}'")),
                        suggestion: Some("Extract to named constant".to_string()),
                        related: Vec::new(),
                        fix: None,
                    });
                }
            }
        }

        diagnostics
    }

    /// Detect magic strings
    fn detect_magic_strings(&self, path: &Path, source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        // Pattern for string literals
        let string_pattern = match regex::Regex::new(r#""([^"]{4,})"#) {
            Ok(re) => re,
            Err(_) => return diagnostics,
        };

        for (line_num, line) in lines.iter().enumerate() {
            // Skip comments
            if line.trim().starts_with("//") || line.trim().starts_with('#') {
                continue;
            }

            for cap in string_pattern.captures_iter(line) {
                if let Some(string) = cap.get(1) {
                    let str_val = string.as_str();

                    // Skip if it looks like a constant definition
                    if line.to_uppercase().contains("CONST") || line.contains("final") {
                        continue;
                    }

                    // Skip common patterns
                    if str_val.is_empty() || str_val.len() < 4 {
                        continue;
                    }

                    diagnostics.push(Diagnostic {
                        severity: CodeSmellType::MagicString.severity(),
                        file: path.to_path_buf(),
                        span: Span {
                            start: line_num as u32 + 1,
                            end: line_num as u32 + 1,
                        },
                        rule_id: CodeSmellType::MagicString.rule_id().to_string(),
                        message: CodeSmellType::MagicString.message(&format!("'{str_val}'")),
                        suggestion: Some("Extract to named constant".to_string()),
                        related: Vec::new(),
                        fix: None,
                    });
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_long_method() {
        let detector = CodeSmellDetector::with_default_config();
        let source = format!("fn test() {{\n{}\n}}", "    println!(\"x\");\n".repeat(60));
        let path = Path::new("test.rs");

        let diagnostics = detector.detect_long_methods(path, &source);
        assert!(!diagnostics.is_empty());
        assert_eq!(diagnostics[0].rule_id, "long-method");
    }

    #[test]
    fn test_detect_too_many_parameters() {
        let detector = CodeSmellDetector::with_default_config();
        let source = "fn test(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32) {}";
        let path = Path::new("test.rs");

        let diagnostics = detector.detect_too_many_parameters(path, source);
        assert!(!diagnostics.is_empty());
        assert_eq!(diagnostics[0].rule_id, "too-many-parameters");
    }

    #[test]
    fn test_detect_deep_nesting() {
        let detector = CodeSmellDetector::with_default_config();
        let source = "if (a) {\n  if (b) {\n    if (c) {\n      if (d) {\n        if (e) {\n          x();\n        }\n      }\n    }\n  }\n}";
        let path = Path::new("test.js");

        let diagnostics = detector.detect_deep_nesting(path, source);
        assert!(!diagnostics.is_empty());
        assert_eq!(diagnostics[0].rule_id, "deep-nesting");
    }

    #[test]
    fn test_detect_magic_number() {
        let detector = CodeSmellDetector::with_default_config();
        let source = "if (age > 18) { return true; }";
        let path = Path::new("test.js");

        let diagnostics = detector.detect_magic_numbers(path, source);
        assert!(!diagnostics.is_empty());
        assert_eq!(diagnostics[0].rule_id, "magic-number");
    }

    #[test]
    fn test_magic_number_exceptions() {
        let detector = CodeSmellDetector::with_default_config();
        let source = "if (count > 0) { return true; }";
        let path = Path::new("test.js");

        let diagnostics = detector.detect_magic_numbers(path, source);
        assert!(diagnostics.is_empty()); // 0 is in exceptions
    }
}
