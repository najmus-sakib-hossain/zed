//! Anti-Pattern Detection
//!
//! Detects architectural anti-patterns that indicate poor design:
//! - God objects/classes (classes with too many responsibilities)
//! - Circular dependencies (modules that depend on each other)
//! - Tight coupling patterns (excessive dependencies between modules)
//!
//! Provides refactoring suggestions for detected anti-patterns.
//! All violations are mapped to the `DesignPatterns` category.

use crate::diagnostics::{Diagnostic, DiagnosticSeverity, RelatedInfo, Span};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Anti-pattern types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AntiPatternType {
    GodObject,
    CircularDependency,
    TightCoupling,
}

impl AntiPatternType {
    #[must_use]
    pub fn rule_id(&self) -> &'static str {
        match self {
            AntiPatternType::GodObject => "god-object",
            AntiPatternType::CircularDependency => "circular-dependency",
            AntiPatternType::TightCoupling => "tight-coupling",
        }
    }

    #[must_use]
    pub fn message(&self, details: &str) -> String {
        match self {
            AntiPatternType::GodObject => format!("God object detected: {details}"),
            AntiPatternType::CircularDependency => {
                format!("Circular dependency detected: {details}")
            }
            AntiPatternType::TightCoupling => format!("Tight coupling detected: {details}"),
        }
    }

    #[must_use]
    pub fn severity(&self) -> DiagnosticSeverity {
        match self {
            AntiPatternType::GodObject => DiagnosticSeverity::Warning,
            AntiPatternType::CircularDependency => DiagnosticSeverity::Error,
            AntiPatternType::TightCoupling => DiagnosticSeverity::Warning,
        }
    }

    #[must_use]
    pub fn suggestion(&self) -> &'static str {
        match self {
            AntiPatternType::GodObject => {
                "Break this class into smaller, focused classes following Single Responsibility Principle"
            }
            AntiPatternType::CircularDependency => {
                "Refactor to break the circular dependency. Consider introducing an interface or moving shared code to a separate module"
            }
            AntiPatternType::TightCoupling => {
                "Reduce coupling by using dependency injection, interfaces, or event-driven patterns"
            }
        }
    }
}

/// Configuration for anti-pattern detection thresholds
#[derive(Debug, Clone)]
pub struct AntiPatternConfig {
    /// Maximum number of methods for a class before it's considered a god object
    pub max_methods: usize,
    /// Maximum number of fields for a class before it's considered a god object
    pub max_fields: usize,
    /// Maximum lines of code for a class before it's considered a god object
    pub max_class_lines: usize,
    /// Maximum number of dependencies for a module before it's considered tightly coupled
    pub max_dependencies: usize,
    /// Maximum number of imports from a single module before it's considered tightly coupled
    pub max_imports_per_module: usize,
}

impl Default for AntiPatternConfig {
    fn default() -> Self {
        Self {
            max_methods: 20,
            max_fields: 15,
            max_class_lines: 500,
            max_dependencies: 10,
            max_imports_per_module: 5,
        }
    }
}

/// Represents a class/struct in the codebase
#[derive(Debug, Clone)]
struct ClassInfo {
    name: String,
    line: usize,
    methods: Vec<String>,
    fields: Vec<String>,
    lines: usize,
}

/// Represents a module and its dependencies
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ModuleInfo {
    path: PathBuf,
    imports: Vec<String>,
    exports: Vec<String>,
}

/// Anti-pattern detector
pub struct AntiPatternDetector {
    config: AntiPatternConfig,
}

impl AntiPatternDetector {
    #[must_use]
    pub fn new(config: AntiPatternConfig) -> Self {
        Self { config }
    }

    #[must_use]
    pub fn with_default_config() -> Self {
        Self::new(AntiPatternConfig::default())
    }

    /// Detect all anti-patterns in a file
    #[must_use]
    pub fn detect(&self, path: &Path, source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Detect god objects
        diagnostics.extend(self.detect_god_objects(path, source));

        diagnostics
    }

    /// Detect all anti-patterns across multiple files (for project-level analysis)
    #[must_use]
    pub fn detect_project(&self, files: &[(PathBuf, String)]) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Build module dependency graph
        let modules = self.build_module_graph(files);

        // Detect circular dependencies
        diagnostics.extend(self.detect_circular_dependencies(&modules));

        // Detect tight coupling
        diagnostics.extend(self.detect_tight_coupling(&modules));

        // Also detect god objects in each file
        for (path, source) in files {
            diagnostics.extend(self.detect_god_objects(path, source));
        }

        diagnostics
    }

    /// Detect god objects (classes with too many responsibilities)
    fn detect_god_objects(&self, path: &Path, source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let classes = self.extract_classes(source);

        for class in classes {
            let mut violations = Vec::new();

            // Check method count
            if class.methods.len() > self.config.max_methods {
                violations.push(format!(
                    "{} methods (max {})",
                    class.methods.len(),
                    self.config.max_methods
                ));
            }

            // Check field count
            if class.fields.len() > self.config.max_fields {
                violations.push(format!(
                    "{} fields (max {})",
                    class.fields.len(),
                    self.config.max_fields
                ));
            }

            // Check lines of code
            if class.lines > self.config.max_class_lines {
                violations
                    .push(format!("{} lines (max {})", class.lines, self.config.max_class_lines));
            }

            // Report if we have any violations (god object is indicated by exceeding thresholds)
            if !violations.is_empty() {
                diagnostics.push(Diagnostic {
                    severity: AntiPatternType::GodObject.severity(),
                    file: path.to_path_buf(),
                    span: Span {
                        start: class.line as u32,
                        end: (class.line + class.lines) as u32,
                    },
                    rule_id: AntiPatternType::GodObject.rule_id().to_string(),
                    message: AntiPatternType::GodObject.message(&format!(
                        "class '{}' has {}",
                        class.name,
                        violations.join(", ")
                    )),
                    suggestion: Some(AntiPatternType::GodObject.suggestion().to_string()),
                    related: Vec::new(),
                    fix: None,
                });
            }
        }

        diagnostics
    }

    /// Extract class information from source code
    fn extract_classes(&self, source: &str) -> Vec<ClassInfo> {
        let mut classes = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        // Patterns for class declarations
        let class_patterns = [
            r"^\s*class\s+(\w+)",          // JavaScript/TypeScript/Python
            r"^\s*struct\s+(\w+)",         // Rust
            r"^\s*type\s+(\w+)\s+struct",  // Go
            r"^\s*public\s+class\s+(\w+)", // Java/C#
            r"^\s*interface\s+(\w+)",      // TypeScript/Java
        ];

        for (line_num, line) in lines.iter().enumerate() {
            for pattern in &class_patterns {
                if let Ok(re) = regex::Regex::new(pattern)
                    && let Some(caps) = re.captures(line)
                    && let Some(name_match) = caps.get(1)
                {
                    let name = name_match.as_str().to_string();
                    let class_lines = self.count_class_lines(&lines, line_num);
                    let methods = self.extract_methods(&lines, line_num, class_lines);
                    let fields = self.extract_fields(&lines, line_num, class_lines);

                    classes.push(ClassInfo {
                        name,
                        line: line_num,
                        methods,
                        fields,
                        lines: class_lines,
                    });
                }
            }
        }

        classes
    }

    /// Count lines in a class/struct
    fn count_class_lines(&self, lines: &[&str], start_line: usize) -> usize {
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

    /// Extract method names from a class
    fn extract_methods(
        &self,
        lines: &[&str],
        start_line: usize,
        class_lines: usize,
    ) -> Vec<String> {
        let mut methods = Vec::new();
        let class_content =
            &lines[start_line..start_line + class_lines.min(lines.len() - start_line)];

        // Patterns for method declarations
        let method_patterns = [
            r"^\s*fn\s+(\w+)",                 // Rust
            r"^\s*function\s+(\w+)",           // JavaScript function
            r"^\s*def\s+(\w+)",                // Python
            r"^\s*func\s+(\w+)",               // Go
            r"^\s*(\w+)\s*\([^)]*\)\s*\{",     // JavaScript/Java methods
            r"^\s*async\s+(\w+)\s*\(",         // Async methods
            r"^\s*public\s+\w+\s+(\w+)\s*\(",  // Java public methods
            r"^\s*private\s+\w+\s+(\w+)\s*\(", // Java private methods
        ];

        for line in class_content {
            for pattern in &method_patterns {
                if let Ok(re) = regex::Regex::new(pattern)
                    && let Some(caps) = re.captures(line)
                    && let Some(name_match) = caps.get(1)
                {
                    let name = name_match.as_str().to_string();
                    // Skip constructors and common keywords
                    if !["if", "for", "while", "switch", "return", "class", "struct"]
                        .contains(&name.as_str())
                    {
                        methods.push(name);
                    }
                }
            }
        }

        methods
    }

    /// Extract field names from a class
    fn extract_fields(&self, lines: &[&str], start_line: usize, class_lines: usize) -> Vec<String> {
        let mut fields = Vec::new();
        let class_content =
            &lines[start_line..start_line + class_lines.min(lines.len() - start_line)];

        // Patterns for field declarations
        let field_patterns = [
            r"^\s*(\w+)\s*:\s*\w+",   // TypeScript/Rust fields
            r"^\s*private\s+(\w+)",   // Java/C# private fields
            r"^\s*public\s+(\w+)",    // Java/C# public fields
            r"^\s*protected\s+(\w+)", // Java/C# protected fields
            r"^\s*self\.(\w+)\s*=",   // Python instance variables
            r"^\s*this\.(\w+)\s*=",   // JavaScript instance variables
        ];

        for line in class_content {
            // Skip method declarations
            if line.contains("function") || line.contains("fn ") || line.contains("def ") {
                continue;
            }

            for pattern in &field_patterns {
                if let Ok(re) = regex::Regex::new(pattern)
                    && let Some(caps) = re.captures(line)
                    && let Some(name_match) = caps.get(1)
                {
                    fields.push(name_match.as_str().to_string());
                }
            }
        }

        fields
    }

    /// Build module dependency graph
    fn build_module_graph(&self, files: &[(PathBuf, String)]) -> HashMap<PathBuf, ModuleInfo> {
        let mut modules = HashMap::new();

        for (path, source) in files {
            let imports = self.extract_imports(source);
            let exports = self.extract_exports(source);

            modules.insert(
                path.clone(),
                ModuleInfo {
                    path: path.clone(),
                    imports,
                    exports,
                },
            );
        }

        modules
    }

    /// Extract import statements from source code
    fn extract_imports(&self, source: &str) -> Vec<String> {
        let mut imports = Vec::new();

        // Patterns for import statements
        let import_patterns = [
            r#"import\s+.*\s+from\s+['"]([^'"]+)['"]"#, // ES6 imports
            r#"require\(['"]([^'"]+)['"]\)"#,           // CommonJS
            r"use\s+([^;]+);",                          // Rust
            r"import\s+([^\s;]+)",                      // Python/Go
            r#"#include\s+[<"]([^>"]+)[>"]"#,           // C/C++
        ];

        for line in source.lines() {
            for pattern in &import_patterns {
                if let Ok(re) = regex::Regex::new(pattern)
                    && let Some(caps) = re.captures(line)
                    && let Some(import_match) = caps.get(1)
                {
                    imports.push(import_match.as_str().to_string());
                }
            }
        }

        imports
    }

    /// Extract export statements from source code
    fn extract_exports(&self, source: &str) -> Vec<String> {
        let mut exports = Vec::new();

        // Patterns for export statements
        let export_patterns = [
            r"export\s+(?:default\s+)?(?:class|function|const|let|var)\s+(\w+)", // ES6 exports
            r"module\.exports\s*=\s*(\w+)",                                      // CommonJS
            r"pub\s+(?:fn|struct|enum|trait)\s+(\w+)",                           // Rust
        ];

        for line in source.lines() {
            for pattern in &export_patterns {
                if let Ok(re) = regex::Regex::new(pattern)
                    && let Some(caps) = re.captures(line)
                    && let Some(export_match) = caps.get(1)
                {
                    exports.push(export_match.as_str().to_string());
                }
            }
        }

        exports
    }

    /// Detect circular dependencies using DFS
    fn detect_circular_dependencies(
        &self,
        modules: &HashMap<PathBuf, ModuleInfo>,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = Vec::new();

        // Build adjacency list for module dependencies
        let mut graph: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
        for (path, module) in modules {
            let mut deps = Vec::new();
            for import in &module.imports {
                // Try to resolve import to actual file path
                if let Some(dep_path) = self.resolve_import(import, modules) {
                    deps.push(dep_path);
                }
            }
            graph.insert(path.clone(), deps);
        }

        // DFS to detect cycles
        for path in modules.keys() {
            if !visited.contains(path)
                && let Some(cycle) =
                    self.dfs_detect_cycle(path, &graph, &mut visited, &mut rec_stack)
            {
                // Found a cycle
                let cycle_str = cycle
                    .iter()
                    .map(|p| p.file_name().unwrap_or_default().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" -> ");

                diagnostics.push(Diagnostic {
                    severity: AntiPatternType::CircularDependency.severity(),
                    file: path.clone(),
                    span: Span { start: 1, end: 1 },
                    rule_id: AntiPatternType::CircularDependency.rule_id().to_string(),
                    message: AntiPatternType::CircularDependency.message(&cycle_str),
                    suggestion: Some(AntiPatternType::CircularDependency.suggestion().to_string()),
                    related: cycle
                        .iter()
                        .map(|p| RelatedInfo {
                            file: p.clone(),
                            span: Span { start: 1, end: 1 },
                            message: "Part of circular dependency".to_string(),
                        })
                        .collect(),
                    fix: None,
                });
            }
        }

        diagnostics
    }

    /// DFS helper to detect cycles
    fn dfs_detect_cycle(
        &self,
        node: &PathBuf,
        graph: &HashMap<PathBuf, Vec<PathBuf>>,
        visited: &mut HashSet<PathBuf>,
        rec_stack: &mut Vec<PathBuf>,
    ) -> Option<Vec<PathBuf>> {
        visited.insert(node.clone());
        rec_stack.push(node.clone());

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if let Some(cycle) = self.dfs_detect_cycle(neighbor, graph, visited, rec_stack)
                    {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(neighbor) {
                    // Found a cycle
                    let cycle_start = rec_stack.iter().position(|p| p == neighbor).unwrap();
                    return Some(rec_stack[cycle_start..].to_vec());
                }
            }
        }

        rec_stack.pop();
        None
    }

    /// Resolve import string to actual file path
    fn resolve_import(
        &self,
        import: &str,
        modules: &HashMap<PathBuf, ModuleInfo>,
    ) -> Option<PathBuf> {
        // Simple resolution: try to find a module that matches the import
        for path in modules.keys() {
            let file_stem = path.file_stem()?.to_string_lossy();
            if import.contains(&*file_stem) || import.ends_with(&*file_stem) {
                return Some(path.clone());
            }
        }
        None
    }

    /// Detect tight coupling (excessive dependencies)
    fn detect_tight_coupling(&self, modules: &HashMap<PathBuf, ModuleInfo>) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for (path, module) in modules {
            // Check total number of dependencies
            if module.imports.len() > self.config.max_dependencies {
                diagnostics.push(Diagnostic {
                    severity: AntiPatternType::TightCoupling.severity(),
                    file: path.clone(),
                    span: Span { start: 1, end: 1 },
                    rule_id: AntiPatternType::TightCoupling.rule_id().to_string(),
                    message: AntiPatternType::TightCoupling.message(&format!(
                        "{} dependencies (max {})",
                        module.imports.len(),
                        self.config.max_dependencies
                    )),
                    suggestion: Some(AntiPatternType::TightCoupling.suggestion().to_string()),
                    related: Vec::new(),
                    fix: None,
                });
            }

            // Check for excessive imports from a single module
            let mut import_counts: HashMap<String, usize> = HashMap::new();
            for import in &module.imports {
                *import_counts.entry(import.clone()).or_insert(0) += 1;
            }

            for (import, count) in import_counts {
                if count > self.config.max_imports_per_module {
                    diagnostics.push(Diagnostic {
                        severity: AntiPatternType::TightCoupling.severity(),
                        file: path.clone(),
                        span: Span { start: 1, end: 1 },
                        rule_id: AntiPatternType::TightCoupling.rule_id().to_string(),
                        message: AntiPatternType::TightCoupling.message(&format!(
                            "{} imports from '{}' (max {})",
                            count, import, self.config.max_imports_per_module
                        )),
                        suggestion: Some(
                            "Consider using a facade pattern or reducing dependencies".to_string(),
                        ),
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
    fn test_detect_god_object_many_methods() {
        let detector = AntiPatternDetector::with_default_config();
        let source = format!(
            "class BigClass {{\n{}\n}}",
            (0..25).map(|i| format!("  method{}() {{}}", i)).collect::<Vec<_>>().join("\n")
        );
        let path = Path::new("test.js");

        let diagnostics = detector.detect_god_objects(path, &source);
        assert!(!diagnostics.is_empty());
        assert_eq!(diagnostics[0].rule_id, "god-object");
    }

    #[test]
    fn test_detect_god_object_many_fields() {
        let detector = AntiPatternDetector::with_default_config();
        let source = format!(
            "class BigClass {{\n{}\n}}",
            (0..20).map(|i| format!("  field{}: number;", i)).collect::<Vec<_>>().join("\n")
        );
        let path = Path::new("test.ts");

        let diagnostics = detector.detect_god_objects(path, &source);
        assert!(!diagnostics.is_empty());
        assert_eq!(diagnostics[0].rule_id, "god-object");
    }

    #[test]
    fn test_no_god_object_small_class() {
        let detector = AntiPatternDetector::with_default_config();
        let source = r#"
class SmallClass {
  field1: number;
  field2: string;
  
  method1() {}
  method2() {}
}
"#;
        let path = Path::new("test.ts");

        let diagnostics = detector.detect_god_objects(path, source);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_detect_circular_dependency() {
        let detector = AntiPatternDetector::with_default_config();
        let files = vec![
            (
                PathBuf::from("a.js"),
                "import { B } from './b.js';\nexport class A {}".to_string(),
            ),
            (
                PathBuf::from("b.js"),
                "import { A } from './a.js';\nexport class B {}".to_string(),
            ),
        ];

        let diagnostics = detector.detect_project(&files);
        let circular_deps: Vec<_> =
            diagnostics.iter().filter(|d| d.rule_id == "circular-dependency").collect();

        assert!(!circular_deps.is_empty());
    }

    #[test]
    fn test_detect_tight_coupling() {
        let detector = AntiPatternDetector::with_default_config();
        let imports = (0..15)
            .map(|i| format!("import {{ Thing{} }} from './module{}.js';", i, i))
            .collect::<Vec<_>>()
            .join("\n");

        let files = vec![(PathBuf::from("main.js"), imports)];

        let diagnostics = detector.detect_project(&files);
        let coupling: Vec<_> =
            diagnostics.iter().filter(|d| d.rule_id == "tight-coupling").collect();

        assert!(!coupling.is_empty());
    }

    #[test]
    fn test_extract_classes() {
        let detector = AntiPatternDetector::with_default_config();
        let source = r#"
class MyClass {
  field1: string;
  
  method1() {}
  method2() {}
}

struct MyStruct {
  field1: i32,
  field2: String,
}
"#;

        let classes = detector.extract_classes(source);
        assert_eq!(classes.len(), 2);
        assert_eq!(classes[0].name, "MyClass");
        assert_eq!(classes[1].name, "MyStruct");
    }

    #[test]
    fn test_extract_imports() {
        let detector = AntiPatternDetector::with_default_config();
        let source = r#"
import { foo } from './foo.js';
import bar from './bar.js';
const baz = require('./baz.js');
use std::collections::HashMap;
"#;

        let imports = detector.extract_imports(source);
        assert!(imports.len() >= 3);
    }
}
