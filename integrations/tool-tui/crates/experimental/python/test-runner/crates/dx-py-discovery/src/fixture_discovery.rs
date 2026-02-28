//! Fixture discovery from Python source files
//!
//! This module implements parsing of @pytest.fixture decorators to extract:
//! - Fixture name
//! - Scope (function, class, module, session)
//! - Autouse flag
//! - Parameters
//! - Dependencies (from function parameters)
//! - Whether the fixture uses yield (has teardown)
//!
//! Requirements: 11.1

use crate::parser::PythonParser;
use dx_py_core::DiscoveryError;
use dx_py_fixture::{FixtureDefinition, FixtureScope};
use std::path::Path;
use tree_sitter::Node;

/// Discovered fixture information before full FixtureDefinition construction
#[derive(Debug, Clone)]
pub struct DiscoveredFixture {
    pub name: String,
    pub line: u32,
    pub scope: FixtureScope,
    pub autouse: bool,
    pub params: Vec<String>,
    pub dependencies: Vec<String>,
    pub is_generator: bool,
}

/// Scanner that finds fixture definitions in Python source
pub struct FixtureDiscovery {
    parser: PythonParser,
}

impl FixtureDiscovery {
    /// Create a new fixture discovery scanner
    pub fn new() -> Result<Self, DiscoveryError> {
        Ok(Self {
            parser: PythonParser::new()?,
        })
    }

    /// Discover all fixtures in a file
    pub fn discover_file(&mut self, path: &Path) -> Result<Vec<FixtureDefinition>, DiscoveryError> {
        let source = std::fs::read_to_string(path)?;
        let discovered = self.discover_source(&source)?;

        Ok(discovered
            .into_iter()
            .map(|d| {
                FixtureDefinition::new(&d.name, path, d.line)
                    .with_scope(d.scope)
                    .with_autouse(d.autouse)
                    .with_dependencies(d.dependencies)
                    .with_generator(d.is_generator)
            })
            .collect())
    }

    /// Discover fixtures from source code
    pub fn discover_source(&mut self, source: &str) -> Result<Vec<DiscoveredFixture>, DiscoveryError> {
        let tree = self.parser.parse(source)?;
        let mut fixtures = Vec::new();
        self.walk_tree(tree.root_node(), source.as_bytes(), &mut fixtures);
        Ok(fixtures)
    }

    fn walk_tree(&self, node: Node, source: &[u8], fixtures: &mut Vec<DiscoveredFixture>) {
        match node.kind() {
            "decorated_definition" => {
                // Check if this is a fixture
                if let Some(fixture) = self.check_decorated_function(node, source) {
                    fixtures.push(fixture);
                }
            }
            "function_definition" => {
                // Check for fixtures without decorators (shouldn't happen, but be defensive)
                // Skip this case as fixtures must have decorators
            }
            _ => {
                // Recurse into children
                for child in node.children(&mut node.walk()) {
                    self.walk_tree(child, source, fixtures);
                }
            }
        }
    }

    fn check_decorated_function(&self, node: Node, source: &[u8]) -> Option<DiscoveredFixture> {
        // Check if any decorator is @pytest.fixture
        let mut is_fixture = false;
        let mut scope = FixtureScope::Function;
        let mut autouse = false;
        let mut params = Vec::new();

        for child in node.children(&mut node.walk()) {
            if child.kind() == "decorator" {
                if let Some((is_fix, fix_scope, fix_autouse, fix_params)) = 
                    self.parse_fixture_decorator(child, source) 
                {
                    if is_fix {
                        is_fixture = true;
                        scope = fix_scope;
                        autouse = fix_autouse;
                        params = fix_params;
                    }
                }
            }
        }

        if !is_fixture {
            return None;
        }

        // Find the function definition
        for child in node.children(&mut node.walk()) {
            if child.kind() == "function_definition" {
                let name = self.get_function_name(child, source)?;
                let line = child.start_position().row as u32 + 1;
                let dependencies = self.get_function_parameters(child, source);
                let is_generator = self.has_yield(child, source);

                return Some(DiscoveredFixture {
                    name,
                    line,
                    scope,
                    autouse,
                    params,
                    dependencies,
                    is_generator,
                });
            }
        }

        None
    }

    /// Parse a decorator to check if it's @pytest.fixture and extract parameters
    /// Returns: (is_fixture, scope, autouse, params)
    fn parse_fixture_decorator(
        &self,
        node: Node,
        source: &[u8],
    ) -> Option<(bool, FixtureScope, bool, Vec<String>)> {
        // Look for decorator content
        for child in node.children(&mut node.walk()) {
            match child.kind() {
                "identifier" => {
                    let name = self.node_text(child, source);
                    if name == "fixture" {
                        // Simple @fixture (no pytest prefix, but valid)
                        return Some((true, FixtureScope::Function, false, Vec::new()));
                    }
                }
                "attribute" => {
                    let name = self.node_text(child, source);
                    if name == "pytest.fixture" {
                        // @pytest.fixture without arguments
                        return Some((true, FixtureScope::Function, false, Vec::new()));
                    }
                }
                "call" => {
                    // Decorator with arguments like @pytest.fixture(scope="module")
                    let func_name = self.get_call_function_name(child, source)?;
                    if func_name == "fixture" || func_name == "pytest.fixture" {
                        let (scope, autouse, params) = self.parse_fixture_args(child, source);
                        return Some((true, scope, autouse, params));
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Get the function name from a call node
    fn get_call_function_name(&self, node: Node, source: &[u8]) -> Option<String> {
        for child in node.children(&mut node.walk()) {
            match child.kind() {
                "identifier" => return Some(self.node_text(child, source)),
                "attribute" => return Some(self.node_text(child, source)),
                _ => {}
            }
        }
        None
    }

    /// Parse fixture decorator arguments to extract scope, autouse, params
    fn parse_fixture_args(&self, call_node: Node, source: &[u8]) -> (FixtureScope, bool, Vec<String>) {
        let mut scope = FixtureScope::Function;
        let mut autouse = false;
        let mut params = Vec::new();

        // Find the argument_list node
        for child in call_node.children(&mut call_node.walk()) {
            if child.kind() == "argument_list" {
                // Parse each argument
                for arg in child.children(&mut child.walk()) {
                    match arg.kind() {
                        "keyword_argument" => {
                            if let Some((key, value)) = self.parse_keyword_argument(arg, source) {
                                match key.as_str() {
                                    "scope" => {
                                        scope = self.parse_scope_value(&value);
                                    }
                                    "autouse" => {
                                        autouse = self.parse_bool_value(&value);
                                    }
                                    "params" => {
                                        params = self.parse_params_value(&value, source);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        (scope, autouse, params)
    }

    /// Parse a keyword argument into (key, value) pair
    fn parse_keyword_argument(&self, node: Node, source: &[u8]) -> Option<(String, String)> {
        let mut key = None;
        let mut value = None;

        for child in node.children(&mut node.walk()) {
            match child.kind() {
                "identifier" => {
                    if key.is_none() {
                        key = Some(self.node_text(child, source));
                    }
                }
                "string" | "true" | "false" | "list" => {
                    value = Some(self.node_text(child, source));
                }
                _ => {}
            }
        }

        Some((key?, value?))
    }

    /// Parse a scope value string into FixtureScope enum
    fn parse_scope_value(&self, value: &str) -> FixtureScope {
        // Remove quotes if present
        let value = value.trim_matches(|c| c == '"' || c == '\'');
        match value {
            "function" => FixtureScope::Function,
            "class" => FixtureScope::Class,
            "module" => FixtureScope::Module,
            "session" => FixtureScope::Session,
            _ => FixtureScope::Function, // Default
        }
    }

    /// Parse a boolean value string
    fn parse_bool_value(&self, value: &str) -> bool {
        value == "true" || value == "True"
    }

    /// Parse a params value (list of values)
    /// Handles formats like: [1, 2, 3], ["a", "b"], [(1, "a"), (2, "b")]
    fn parse_params_value(&self, value: &str, _source: &[u8]) -> Vec<String> {
        let trimmed = value.trim();
        
        // Check if it's a list
        if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
            return Vec::new();
        }
        
        // Extract content between brackets
        let inner = &trimmed[1..trimmed.len() - 1];
        
        // Parse list items handling nested structures
        self.split_params_list(inner)
    }
    
    /// Split a params list string into individual items
    /// Handles nested tuples, strings, and basic values
    fn split_params_list(&self, s: &str) -> Vec<String> {
        let mut items = Vec::new();
        let mut current = String::new();
        let mut depth = 0;
        let mut in_string = false;
        let mut string_char = '"';
        
        for c in s.chars() {
            match c {
                '"' | '\'' if !in_string => {
                    in_string = true;
                    string_char = c;
                    current.push(c);
                }
                c if in_string && c == string_char => {
                    in_string = false;
                    current.push(c);
                }
                '(' | '[' | '{' if !in_string => {
                    depth += 1;
                    current.push(c);
                }
                ')' | ']' | '}' if !in_string => {
                    depth -= 1;
                    current.push(c);
                }
                ',' if depth == 0 && !in_string => {
                    let trimmed = current.trim().to_string();
                    if !trimmed.is_empty() {
                        items.push(trimmed);
                    }
                    current.clear();
                }
                _ => current.push(c),
            }
        }
        
        // Don't forget the last item
        let trimmed = current.trim().to_string();
        if !trimmed.is_empty() {
            items.push(trimmed);
        }
        
        items
    }

    /// Get function name from function_definition node
    fn get_function_name(&self, node: Node, source: &[u8]) -> Option<String> {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "identifier" {
                return Some(self.node_text(child, source));
            }
        }
        None
    }

    /// Get function parameters (dependencies) from function_definition node
    fn get_function_parameters(&self, node: Node, source: &[u8]) -> Vec<String> {
        let mut params = Vec::new();

        // Find the parameters node
        for child in node.children(&mut node.walk()) {
            if child.kind() == "parameters" {
                // Extract parameter names
                for param in child.children(&mut child.walk()) {
                    match param.kind() {
                        "identifier" => {
                            params.push(self.node_text(param, source));
                        }
                        "typed_parameter" | "default_parameter" => {
                            // Get the identifier from typed/default parameter
                            for p_child in param.children(&mut param.walk()) {
                                if p_child.kind() == "identifier" {
                                    params.push(self.node_text(p_child, source));
                                    break;
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

    /// Check if a function contains a yield statement (is a generator)
    fn has_yield(&self, node: Node, _source: &[u8]) -> bool {
        // Recursively search for yield_statement or expression_statement with yield
        self.contains_yield_recursive(node)
    }

    fn contains_yield_recursive(&self, node: Node) -> bool {
        if node.kind() == "yield" || node.kind() == "yield_statement" {
            return true;
        }

        for child in node.children(&mut node.walk()) {
            if self.contains_yield_recursive(child) {
                return true;
            }
        }

        false
    }

    /// Get text content of a node
    fn node_text(&self, node: Node, source: &[u8]) -> String {
        let start = node.start_byte();
        let end = node.end_byte();
        String::from_utf8_lossy(&source[start..end]).to_string()
    }
}

impl Default for FixtureDiscovery {
    fn default() -> Self {
        Self::new().expect("Failed to create FixtureDiscovery")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_simple_fixture() {
        let source = r#"
import pytest

@pytest.fixture
def sample_data():
    return {"key": "value"}
"#;

        let mut discovery = FixtureDiscovery::new().unwrap();
        let fixtures = discovery.discover_source(source).unwrap();

        assert_eq!(fixtures.len(), 1);
        assert_eq!(fixtures[0].name, "sample_data");
        assert_eq!(fixtures[0].scope, FixtureScope::Function);
        assert!(!fixtures[0].autouse);
        assert!(!fixtures[0].is_generator);
    }

    #[test]
    fn test_discover_fixture_with_scope() {
        let source = r#"
import pytest

@pytest.fixture(scope="module")
def module_data():
    return {"module": "test"}
"#;

        let mut discovery = FixtureDiscovery::new().unwrap();
        let fixtures = discovery.discover_source(source).unwrap();

        assert_eq!(fixtures.len(), 1);
        assert_eq!(fixtures[0].name, "module_data");
        assert_eq!(fixtures[0].scope, FixtureScope::Module);
    }

    #[test]
    fn test_discover_fixture_with_autouse() {
        let source = r#"
import pytest

@pytest.fixture(autouse=True)
def auto_fixture():
    print("Auto setup")
"#;

        let mut discovery = FixtureDiscovery::new().unwrap();
        let fixtures = discovery.discover_source(source).unwrap();

        assert_eq!(fixtures.len(), 1);
        assert_eq!(fixtures[0].name, "auto_fixture");
        assert!(fixtures[0].autouse);
    }

    #[test]
    fn test_discover_fixture_with_yield() {
        let source = r#"
import pytest

@pytest.fixture
def temp_resource():
    resource = setup()
    yield resource
    teardown(resource)
"#;

        let mut discovery = FixtureDiscovery::new().unwrap();
        let fixtures = discovery.discover_source(source).unwrap();

        assert_eq!(fixtures.len(), 1);
        assert_eq!(fixtures[0].name, "temp_resource");
        assert!(fixtures[0].is_generator);
    }

    #[test]
    fn test_discover_fixture_with_dependencies() {
        let source = r#"
import pytest

@pytest.fixture
def dependent_fixture(sample_data, other_fixture):
    return sample_data + other_fixture
"#;

        let mut discovery = FixtureDiscovery::new().unwrap();
        let fixtures = discovery.discover_source(source).unwrap();

        assert_eq!(fixtures.len(), 1);
        assert_eq!(fixtures[0].name, "dependent_fixture");
        assert_eq!(fixtures[0].dependencies, vec!["sample_data", "other_fixture"]);
    }

    #[test]
    fn test_discover_multiple_fixtures() {
        let source = r#"
import pytest

@pytest.fixture
def fixture_one():
    return 1

@pytest.fixture(scope="session")
def fixture_two():
    return 2

@pytest.fixture(autouse=True)
def fixture_three():
    pass
"#;

        let mut discovery = FixtureDiscovery::new().unwrap();
        let fixtures = discovery.discover_source(source).unwrap();

        assert_eq!(fixtures.len(), 3);
        assert_eq!(fixtures[0].name, "fixture_one");
        assert_eq!(fixtures[1].name, "fixture_two");
        assert_eq!(fixtures[1].scope, FixtureScope::Session);
        assert_eq!(fixtures[2].name, "fixture_three");
        assert!(fixtures[2].autouse);
    }

    #[test]
    fn test_discover_fixture_all_scopes() {
        let source = r#"
import pytest

@pytest.fixture(scope="function")
def func_fixture():
    pass

@pytest.fixture(scope="class")
def class_fixture():
    pass

@pytest.fixture(scope="module")
def module_fixture():
    pass

@pytest.fixture(scope="session")
def session_fixture():
    pass
"#;

        let mut discovery = FixtureDiscovery::new().unwrap();
        let fixtures = discovery.discover_source(source).unwrap();

        assert_eq!(fixtures.len(), 4);
        assert_eq!(fixtures[0].scope, FixtureScope::Function);
        assert_eq!(fixtures[1].scope, FixtureScope::Class);
        assert_eq!(fixtures[2].scope, FixtureScope::Module);
        assert_eq!(fixtures[3].scope, FixtureScope::Session);
    }

    #[test]
    fn test_discover_fixture_with_params() {
        let source = r#"
import pytest

@pytest.fixture(params=[1, 2, 3])
def number_fixture(request):
    return request.param

@pytest.fixture(params=["a", "b", "c"])
def string_fixture(request):
    return request.param

@pytest.fixture(params=[(1, "one"), (2, "two")])
def tuple_fixture(request):
    return request.param
"#;

        let mut discovery = FixtureDiscovery::new().unwrap();
        let fixtures = discovery.discover_source(source).unwrap();

        assert_eq!(fixtures.len(), 3);
        
        // Verify params are parsed
        assert_eq!(fixtures[0].name, "number_fixture");
        assert_eq!(fixtures[0].params.len(), 3);
        assert_eq!(fixtures[0].params[0], "1");
        assert_eq!(fixtures[0].params[1], "2");
        assert_eq!(fixtures[0].params[2], "3");

        assert_eq!(fixtures[1].name, "string_fixture");
        assert_eq!(fixtures[1].params.len(), 3);

        assert_eq!(fixtures[2].name, "tuple_fixture");
        assert_eq!(fixtures[2].params.len(), 2);
        // Tuples should be preserved as strings
        assert!(fixtures[2].params[0].contains("1"));
    }

    #[test]
    fn test_split_params_list() {
        let discovery = FixtureDiscovery::new().unwrap();
        
        // Simple numbers
        let params = discovery.split_params_list("1, 2, 3");
        assert_eq!(params, vec!["1", "2", "3"]);
        
        // Strings with quotes
        let params = discovery.split_params_list(r#""a", "b", "c""#);
        assert_eq!(params.len(), 3);
        
        // Tuples
        let params = discovery.split_params_list(r#"(1, "one"), (2, "two")"#);
        assert_eq!(params.len(), 2);
        assert!(params[0].starts_with('('));
        
        // Nested lists
        let params = discovery.split_params_list("[1, 2], [3, 4]");
        assert_eq!(params.len(), 2);
    }
}
