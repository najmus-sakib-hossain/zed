//! Complexity Metrics Calculation
//!
//! Calculates quantitative complexity metrics for functions/methods:
//! - Cyclomatic complexity (`McCabe`)
//! - Cognitive complexity
//! - Lines of code (LOC, SLOC)
//!
//! These metrics are stored with violations for reporting.

/// Complexity metrics for a function or method
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComplexityMetrics {
    /// Cyclomatic complexity (`McCabe`) - number of linearly independent paths
    pub cyclomatic: usize,
    /// Cognitive complexity - how difficult code is to understand
    pub cognitive: usize,
    /// Total lines of code
    pub loc: usize,
    /// Source lines of code (non-blank, non-comment)
    pub sloc: usize,
    /// Function/method name
    pub name: String,
    /// Start line number
    pub start_line: usize,
    /// End line number
    pub end_line: usize,
}

impl ComplexityMetrics {
    #[must_use]
    pub fn new(name: String, start_line: usize, end_line: usize) -> Self {
        Self {
            cyclomatic: 1, // Base complexity is 1
            cognitive: 0,
            loc: 0,
            sloc: 0,
            name,
            start_line,
            end_line,
        }
    }

    /// Check if metrics exceed thresholds
    #[must_use]
    pub fn is_complex(&self, max_cyclomatic: usize, max_cognitive: usize) -> bool {
        self.cyclomatic > max_cyclomatic || self.cognitive > max_cognitive
    }
}

/// Complexity calculator
#[allow(dead_code)]
pub struct ComplexityCalculator {
    max_cyclomatic: usize,
    max_cognitive: usize,
}

impl ComplexityCalculator {
    #[must_use]
    pub fn new(max_cyclomatic: usize, max_cognitive: usize) -> Self {
        Self {
            max_cyclomatic,
            max_cognitive,
        }
    }

    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(10, 15)
    }

    /// Calculate complexity metrics for all functions in source code
    #[must_use]
    pub fn calculate_all(&self, source: &str) -> Vec<ComplexityMetrics> {
        let mut metrics = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        // Find all function declarations
        for (line_num, line) in lines.iter().enumerate() {
            if let Some(func_name) = self.extract_function_name(line)
                && let Some(func_metrics) =
                    self.calculate_function_metrics(&lines, line_num, &func_name)
            {
                metrics.push(func_metrics);
            }
        }

        metrics
    }

    /// Calculate metrics for a single function
    fn calculate_function_metrics(
        &self,
        lines: &[&str],
        start_line: usize,
        name: &str,
    ) -> Option<ComplexityMetrics> {
        let end_line = self.find_function_end(lines, start_line)?;
        let func_lines = &lines[start_line..=end_line];

        let mut metrics = ComplexityMetrics::new(name.to_string(), start_line, end_line);

        // Calculate LOC and SLOC
        metrics.loc = func_lines.len();
        metrics.sloc = self.count_sloc(func_lines);

        // Calculate cyclomatic complexity
        metrics.cyclomatic = self.calculate_cyclomatic(func_lines);

        // Calculate cognitive complexity
        metrics.cognitive = self.calculate_cognitive(func_lines);

        Some(metrics)
    }

    /// Calculate cyclomatic complexity (`McCabe`)
    /// Formula: M = E - N + 2P where E=edges, N=nodes, P=connected components
    /// Simplified: Count decision points + 1
    fn calculate_cyclomatic(&self, lines: &[&str]) -> usize {
        let mut complexity = 1; // Base complexity

        for line in lines {
            let trimmed = line.trim();

            // Skip comments
            if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*") {
                continue;
            }

            // Count decision points
            complexity += self.count_decision_points(trimmed);
        }

        complexity
    }

    /// Count decision points in a line
    fn count_decision_points(&self, line: &str) -> usize {
        let mut count = 0;

        // Control flow keywords
        let keywords = [
            "if", "else if", "elif", "for", "while", "case", "catch", "&&", "||", "?",
        ];

        for keyword in &keywords {
            // Count occurrences, but be careful with substrings
            let mut pos = 0;
            while let Some(idx) = line[pos..].find(keyword) {
                let abs_pos = pos + idx;

                // Check if it's a whole word (for keywords)
                let is_word_boundary = if keyword.chars().all(char::is_alphabetic) {
                    let before_ok = abs_pos == 0
                        || !line.chars().nth(abs_pos - 1).unwrap_or(' ').is_alphanumeric();
                    let after_ok = abs_pos + keyword.len() >= line.len()
                        || !line
                            .chars()
                            .nth(abs_pos + keyword.len())
                            .unwrap_or(' ')
                            .is_alphanumeric();
                    before_ok && after_ok
                } else {
                    true // Operators don't need word boundaries
                };

                if is_word_boundary {
                    count += 1;
                }

                pos = abs_pos + keyword.len();
            }
        }

        count
    }

    /// Calculate cognitive complexity
    /// Measures how difficult code is to understand (considers nesting)
    fn calculate_cognitive(&self, lines: &[&str]) -> usize {
        let mut complexity = 0;
        let mut nesting_level = 0;
        let mut in_comment = false;

        for line in lines {
            let trimmed = line.trim();

            // Handle multi-line comments
            if trimmed.starts_with("/*") {
                in_comment = true;
            }
            if trimmed.ends_with("*/") {
                in_comment = false;
                continue;
            }
            if in_comment || trimmed.starts_with("//") || trimmed.starts_with('#') {
                continue;
            }

            // Track nesting level
            let open_braces = trimmed.chars().filter(|&c| c == '{').count();
            let close_braces = trimmed.chars().filter(|&c| c == '}').count();

            // Add complexity for control structures (weighted by nesting)
            if self.is_control_structure(trimmed) {
                complexity += 1 + nesting_level;
            }

            // Add complexity for logical operators (not weighted by nesting)
            complexity += self.count_logical_operators(trimmed);

            // Update nesting level
            nesting_level += open_braces;
            nesting_level = nesting_level.saturating_sub(close_braces);
        }

        complexity
    }

    /// Check if line contains a control structure
    fn is_control_structure(&self, line: &str) -> bool {
        let keywords = [
            "if", "else", "for", "while", "switch", "case", "catch", "except",
        ];

        for keyword in &keywords {
            if line.contains(keyword) {
                // Simple check - could be improved with proper parsing
                return true;
            }
        }

        false
    }

    /// Count logical operators (&&, ||)
    fn count_logical_operators(&self, line: &str) -> usize {
        line.matches("&&").count() + line.matches("||").count()
    }

    /// Count source lines of code (non-blank, non-comment)
    fn count_sloc(&self, lines: &[&str]) -> usize {
        let mut count = 0;
        let mut in_multiline_comment = false;

        for line in lines {
            let trimmed = line.trim();

            // Handle multi-line comments
            if trimmed.starts_with("/*") || trimmed.starts_with("/**") {
                in_multiline_comment = true;
            }
            if trimmed.ends_with("*/") {
                in_multiline_comment = false;
                continue;
            }

            // Skip if in comment or blank or single-line comment
            if in_multiline_comment
                || trimmed.is_empty()
                || trimmed.starts_with("//")
                || trimmed.starts_with('#')
            {
                continue;
            }

            count += 1;
        }

        count
    }

    /// Extract function name from declaration line
    fn extract_function_name(&self, line: &str) -> Option<String> {
        let patterns = [
            (r"fn\s+(\w+)", 1),             // Rust
            (r"function\s+(\w+)", 1),       // JavaScript
            (r"def\s+(\w+)", 1),            // Python
            (r"func\s+(\w+)", 1),           // Go
            (r"(\w+)\s*\([^)]*\)\s*\{", 1), // C/C++/Java (simplified)
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

    /// Find the end line of a function starting from declaration
    fn find_function_end(&self, lines: &[&str], start_line: usize) -> Option<usize> {
        let mut brace_count = 0;
        let mut started = false;

        for (offset, line) in lines.iter().enumerate().skip(start_line) {
            for ch in line.chars() {
                if ch == '{' {
                    brace_count += 1;
                    started = true;
                } else if ch == '}' {
                    brace_count -= 1;
                    if started && brace_count == 0 {
                        return Some(offset);
                    }
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cyclomatic_simple() {
        let calc = ComplexityCalculator::with_defaults();
        let source = r#"
fn simple() {
    println!("hello");
}
"#;
        let metrics = calc.calculate_all(source);
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].cyclomatic, 1); // No branches
    }

    #[test]
    fn test_cyclomatic_with_if() {
        let calc = ComplexityCalculator::with_defaults();
        let source = r#"
fn with_if(x: i32) {
    if x > 0 {
        println!("positive");
    }
}
"#;
        let metrics = calc.calculate_all(source);
        assert_eq!(metrics.len(), 1);
        assert!(metrics[0].cyclomatic >= 2); // 1 base + 1 if
    }

    #[test]
    fn test_cyclomatic_complex() {
        let calc = ComplexityCalculator::with_defaults();
        let source = r#"
fn complex(x: i32, y: i32) {
    if x > 0 {
        if y > 0 {
            println!("both positive");
        }
    } else if x < 0 {
        println!("x negative");
    }
    
    for i in 0..10 {
        println!("{}", i);
    }
}
"#;
        let metrics = calc.calculate_all(source);
        assert_eq!(metrics.len(), 1);
        // 1 base + 2 if + 1 else if + 1 for = 5
        assert!(metrics[0].cyclomatic >= 4);
    }

    #[test]
    fn test_cognitive_nesting() {
        let calc = ComplexityCalculator::with_defaults();
        let source = r#"
fn nested(x: i32) {
    if x > 0 {
        if x > 10 {
            println!("big");
        }
    }
}
"#;
        let metrics = calc.calculate_all(source);
        assert_eq!(metrics.len(), 1);
        // First if: 1, nested if: 1 + 1 (nesting) = 2, total = 3
        assert!(metrics[0].cognitive >= 2);
    }

    #[test]
    fn test_loc_sloc() {
        let calc = ComplexityCalculator::with_defaults();
        let source = r#"
fn test() {
    // This is a comment
    let x = 1;
    
    let y = 2;
}
"#;
        let metrics = calc.calculate_all(source);
        assert_eq!(metrics.len(), 1);
        assert!(metrics[0].loc >= 5);
        assert!(metrics[0].sloc >= 2); // Only the two let statements
    }

    #[test]
    fn test_multiple_functions() {
        let calc = ComplexityCalculator::with_defaults();
        let source = r#"
fn first() {
    println!("first");
}

fn second() {
    println!("second");
}
"#;
        let metrics = calc.calculate_all(source);
        assert_eq!(metrics.len(), 2);
        assert_eq!(metrics[0].name, "first");
        assert_eq!(metrics[1].name, "second");
    }

    #[test]
    fn test_is_complex() {
        let metrics = ComplexityMetrics {
            cyclomatic: 15,
            cognitive: 20,
            loc: 100,
            sloc: 80,
            name: "complex_func".to_string(),
            start_line: 0,
            end_line: 100,
        };

        assert!(metrics.is_complex(10, 15));
        assert!(!metrics.is_complex(20, 25));
    }

    #[test]
    fn test_logical_operators() {
        let calc = ComplexityCalculator::with_defaults();
        let source = r#"
fn with_logic(x: i32, y: i32) {
    if x > 0 && y > 0 {
        println!("both");
    }
    if x > 0 || y > 0 {
        println!("at least one");
    }
}
"#;
        let metrics = calc.calculate_all(source);
        assert_eq!(metrics.len(), 1);
        // Should count the logical operators
        assert!(metrics[0].cognitive >= 2);
    }
}
