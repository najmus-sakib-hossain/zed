//! Test scanner that walks the AST to find test functions

use crate::parser::PythonParser;
use dx_py_core::{DiscoveryError, Marker, TestCase};
use std::path::Path;
use tree_sitter::Node;

/// A discovered test before full TestCase construction
#[derive(Debug, Clone)]
pub struct DiscoveredTest {
    pub name: String,
    pub line: u32,
    pub class_name: Option<String>,
    pub markers: Vec<Marker>,
    pub is_fixture: bool,
    pub parameters: Vec<String>,
}

/// Scanner that finds test functions in Python source
pub struct TestScanner {
    parser: PythonParser,
}

impl TestScanner {
    pub fn new() -> Result<Self, DiscoveryError> {
        Ok(Self {
            parser: PythonParser::new()?,
        })
    }

    /// Scan a file for test functions
    pub fn scan_file(&mut self, path: &Path) -> Result<Vec<TestCase>, DiscoveryError> {
        let source = std::fs::read_to_string(path)?;
        let discovered = self.scan_source(&source)?;

        Ok(discovered
            .into_iter()
            .filter(|d| !d.is_fixture)
            .map(|d| {
                let mut tc = TestCase::new(&d.name, path, d.line);
                if let Some(class) = d.class_name {
                    tc = tc.with_class(class);
                }
                for marker in d.markers {
                    tc = tc.with_marker(marker);
                }
                tc = tc.with_parameters(d.parameters);
                tc
            })
            .collect())
    }

    /// Scan source code for test functions
    pub fn scan_source(&mut self, source: &str) -> Result<Vec<DiscoveredTest>, DiscoveryError> {
        let tree = self.parser.parse(source)?;
        let mut tests = Vec::new();
        self.walk_tree(tree.root_node(), source.as_bytes(), None, &mut tests);
        Ok(tests)
    }

    fn walk_tree(
        &self,
        node: Node,
        source: &[u8],
        current_class: Option<&str>,
        tests: &mut Vec<DiscoveredTest>,
    ) {
        match node.kind() {
            "function_definition" => {
                if let Some(test) = self.check_function(node, source, current_class) {
                    tests.push(test);
                }
            }
            "class_definition" => {
                if let Some(class_name) = self.get_class_name(node, source) {
                    // Only scan methods if class starts with "Test"
                    if class_name.starts_with("Test") {
                        // Find the class body and scan it
                        for child in node.children(&mut node.walk()) {
                            if child.kind() == "block" {
                                self.walk_tree(child, source, Some(&class_name), tests);
                            }
                        }
                    }
                }
            }
            _ => {
                // Recurse into children
                for child in node.children(&mut node.walk()) {
                    self.walk_tree(child, source, current_class, tests);
                }
            }
        }
    }

    fn check_function(
        &self,
        node: Node,
        source: &[u8],
        current_class: Option<&str>,
    ) -> Option<DiscoveredTest> {
        let name = self.get_function_name(node, source)?;
        let markers = self.get_decorators(node, source);
        let is_fixture = markers.iter().any(|m| m.name == "fixture" || m.name == "pytest.fixture");
        let parameters = self.get_function_parameters(node, source);

        // Check if it's a test function
        let is_test = name.starts_with("test_")
            || name.ends_with("_test")
            || markers.iter().any(|m| m.name.starts_with("pytest.mark"));

        if is_test || is_fixture {
            Some(DiscoveredTest {
                name,
                line: node.start_position().row as u32 + 1, // 1-indexed
                class_name: current_class.map(String::from),
                markers,
                is_fixture,
                parameters,
            })
        } else {
            None
        }
    }

    fn get_function_name(&self, node: Node, source: &[u8]) -> Option<String> {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "identifier" {
                return Some(self.node_text(child, source));
            }
        }
        None
    }

    fn get_class_name(&self, node: Node, source: &[u8]) -> Option<String> {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "identifier" {
                return Some(self.node_text(child, source));
            }
        }
        None
    }

    fn get_decorators(&self, node: Node, source: &[u8]) -> Vec<Marker> {
        let mut markers = Vec::new();

        // Look for decorator nodes that are siblings before this function
        if let Some(parent) = node.parent() {
            let mut cursor = parent.walk();
            for child in parent.children(&mut cursor) {
                if child.kind() == "decorator" {
                    if let Some(marker) = self.parse_decorator(child, source) {
                        markers.push(marker);
                    }
                }
                if child.id() == node.id() {
                    break;
                }
            }
        }

        // Also check if this node is inside a decorated_definition
        if let Some(parent) = node.parent() {
            if parent.kind() == "decorated_definition" {
                for child in parent.children(&mut parent.walk()) {
                    if child.kind() == "decorator" {
                        if let Some(marker) = self.parse_decorator(child, source) {
                            markers.push(marker);
                        }
                    }
                }
            }
        }

        markers
    }

    fn parse_decorator(&self, node: Node, source: &[u8]) -> Option<Marker> {
        // Find the decorator name (could be simple identifier or attribute access)
        for child in node.children(&mut node.walk()) {
            match child.kind() {
                "identifier" => {
                    return Some(Marker::new(self.node_text(child, source)));
                }
                "attribute" => {
                    return Some(Marker::new(self.node_text(child, source)));
                }
                "call" => {
                    // Decorator with arguments like @pytest.mark.parametrize(...)
                    for call_child in child.children(&mut child.walk()) {
                        if call_child.kind() == "attribute" || call_child.kind() == "identifier" {
                            let name = self.node_text(call_child, source);
                            let args = self.get_call_args(child, source);
                            return Some(Marker::with_args(name, args));
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn get_call_args(&self, node: Node, source: &[u8]) -> Vec<String> {
        let mut args = Vec::new();
        for child in node.children(&mut node.walk()) {
            if child.kind() == "argument_list" {
                for arg in child.children(&mut child.walk()) {
                    if arg.kind() != "(" && arg.kind() != ")" && arg.kind() != "," {
                        args.push(self.node_text(arg, source));
                    }
                }
            }
        }
        args
    }

    /// Get function parameters from function_definition node
    fn get_function_parameters(&self, node: Node, source: &[u8]) -> Vec<String> {
        let mut params = Vec::new();

        // Find the parameters node
        for child in node.children(&mut node.walk()) {
            if child.kind() == "parameters" {
                // Extract parameter names
                for param in child.children(&mut child.walk()) {
                    match param.kind() {
                        "identifier" => {
                            let param_name = self.node_text(param, source);
                            // Skip 'self' and 'cls' parameters
                            if param_name != "self" && param_name != "cls" {
                                params.push(param_name);
                            }
                        }
                        "typed_parameter" | "default_parameter" => {
                            // Get the identifier from typed/default parameter
                            for p_child in param.children(&mut param.walk()) {
                                if p_child.kind() == "identifier" {
                                    let param_name = self.node_text(p_child, source);
                                    // Skip 'self' and 'cls' parameters
                                    if param_name != "self" && param_name != "cls" {
                                        params.push(param_name);
                                        break;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        params
    }

    fn node_text(&self, node: Node, source: &[u8]) -> String {
        let start = node.start_byte();
        let end = node.end_byte();
        String::from_utf8_lossy(&source[start..end]).to_string()
    }
}

impl Default for TestScanner {
    fn default() -> Self {
        Self::new().expect("Failed to create TestScanner")
    }
}
