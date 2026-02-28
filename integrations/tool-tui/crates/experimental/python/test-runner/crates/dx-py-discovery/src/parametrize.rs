//! Parametrization support for pytest-compatible test expansion
//!
//! This module implements:
//! - ParametrizeExpander for expanding parametrized tests
//! - Cartesian product of multiple @pytest.mark.parametrize decorators
//! - Custom IDs and xfail markers support

use dx_py_core::{Marker, TestCase, TestId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single parameter set from @pytest.mark.parametrize
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParameterSet {
    /// Parameter names (e.g., ["x", "y"] from parametrize("x,y", [...]))
    pub argnames: Vec<String>,
    /// Parameter values - each inner Vec is one set of values
    pub values: Vec<Vec<String>>,
    /// Optional custom IDs for each parameter set
    pub ids: Vec<Option<String>>,
    /// Marks for each parameter set (e.g., xfail, skip)
    pub marks: Vec<Vec<String>>,
    /// Whether parameters should be passed through fixtures (indirect=True)
    /// Can be true for all params, or a list of specific param names
    pub indirect: IndirectMode,
}

/// Specifies how indirect parameter routing works
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum IndirectMode {
    /// No indirect routing (default)
    #[default]
    None,
    /// All parameters are indirect
    All,
    /// Only specific parameters are indirect
    Specific(Vec<String>),
}

impl IndirectMode {
    /// Check if a parameter name should be routed through a fixture
    pub fn is_indirect(&self, param_name: &str) -> bool {
        match self {
            IndirectMode::None => false,
            IndirectMode::All => true,
            IndirectMode::Specific(names) => names.iter().any(|n| n == param_name),
        }
    }
}

impl ParameterSet {
    /// Create a new parameter set
    pub fn new(argnames: Vec<String>) -> Self {
        Self {
            argnames,
            values: Vec::new(),
            ids: Vec::new(),
            marks: Vec::new(),
            indirect: IndirectMode::None,
        }
    }

    /// Set the indirect mode for this parameter set
    pub fn with_indirect(mut self, indirect: IndirectMode) -> Self {
        self.indirect = indirect;
        self
    }

    /// Add a set of values with optional ID and marks
    pub fn add_values(
        mut self,
        values: Vec<String>,
        id: Option<String>,
        marks: Vec<String>,
    ) -> Self {
        self.values.push(values);
        self.ids.push(id);
        self.marks.push(marks);
        self
    }

    /// Get the number of parameter sets
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Expanded test with parameter values
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpandedTest {
    /// Original test case
    pub base: TestCase,
    /// Parameter values for this variant
    pub param_values: HashMap<String, String>,
    /// Parameter names that should be routed through fixtures (indirect)
    pub indirect_params: Vec<String>,
    /// Generated test ID suffix (e.g., "[0-1]" or "[custom_id]")
    pub id_suffix: String,
    /// Whether this test is expected to fail
    pub expected_failure: bool,
    /// Skip reason if marked to skip
    pub skip_reason: Option<String>,
}

impl ExpandedTest {
    /// Check if a parameter is indirect (should be routed through a fixture)
    pub fn is_indirect(&self, param_name: &str) -> bool {
        self.indirect_params.iter().any(|n| n == param_name)
    }

    /// Get the full test ID including parameters
    pub fn full_id(&self) -> String {
        format!("{}[{}]", self.base.full_name(), self.id_suffix)
    }

    /// Convert to a TestCase with updated ID
    pub fn to_test_case(&self) -> TestCase {
        let mut tc = self.base.clone();
        // Update the name to include parameter suffix
        tc.name = format!("{}[{}]", tc.name, self.id_suffix);
        // Recalculate ID based on new name
        let file_hash = blake3::hash(tc.file_path.to_string_lossy().as_bytes()).as_bytes()[0..8]
            .iter()
            .fold(0u64, |acc, &b| (acc << 8) | b as u64);
        let name_hash = blake3::hash(tc.name.as_bytes()).as_bytes()[0..8]
            .iter()
            .fold(0u64, |acc, &b| (acc << 8) | b as u64);
        tc.id = TestId::new(file_hash, tc.line_number, name_hash);

        // Add xfail marker if expected to fail
        if self.expected_failure {
            tc = tc.with_marker(Marker::new("xfail"));
        }

        // Add skip marker if has skip reason
        if let Some(reason) = &self.skip_reason {
            tc = tc.with_marker(Marker::with_args("skip", vec![reason.clone()]));
        }

        tc
    }
}

/// Expands parametrized tests into multiple test variants
pub struct ParametrizeExpander;

impl ParametrizeExpander {
    /// Create a new expander
    pub fn new() -> Self {
        Self
    }

    /// Expand a test case based on its parametrize markers
    ///
    /// If the test has no parametrize markers, returns a single-element vector
    /// with the original test wrapped in ExpandedTest.
    ///
    /// If the test has multiple parametrize decorators, generates the cartesian
    /// product of all parameter sets.
    pub fn expand(&self, test: &TestCase) -> Vec<ExpandedTest> {
        let param_sets = self.extract_parameter_sets(test);

        if param_sets.is_empty() {
            // No parametrization - return original test
            return vec![ExpandedTest {
                base: test.clone(),
                param_values: HashMap::new(),
                indirect_params: Vec::new(),
                id_suffix: String::new(),
                expected_failure: false,
                skip_reason: None,
            }];
        }

        // Generate cartesian product of all parameter sets
        self.generate_cartesian_product(test, &param_sets)
    }

    /// Extract parameter sets from test markers
    fn extract_parameter_sets(&self, test: &TestCase) -> Vec<ParameterSet> {
        let mut param_sets = Vec::new();

        for marker in &test.markers {
            if marker.name == "pytest.mark.parametrize" || marker.name == "parametrize" {
                if let Some(param_set) = self.parse_parametrize_marker(marker) {
                    param_sets.push(param_set);
                }
            }
        }

        param_sets
    }

    /// Parse a parametrize marker into a ParameterSet
    fn parse_parametrize_marker(&self, marker: &Marker) -> Option<ParameterSet> {
        if marker.args.len() < 2 {
            return None;
        }

        // First arg is the parameter names (e.g., "x,y" or "x")
        let argnames_str = marker.args[0].trim_matches(|c| c == '"' || c == '\'');
        let argnames: Vec<String> = argnames_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if argnames.is_empty() {
            return None;
        }

        // Second arg is the values list
        let values_str = &marker.args[1];
        let (values, ids, marks) = self.parse_values_list(values_str, argnames.len());

        // Check for indirect argument in kwargs
        let indirect = self.parse_indirect_kwarg(marker);

        Some(ParameterSet {
            argnames,
            values,
            ids,
            marks,
            indirect,
        })
    }

    /// Parse the indirect keyword argument from a parametrize marker
    /// Handles: indirect=True, indirect=False, indirect=["param1", "param2"]
    fn parse_indirect_kwarg(&self, marker: &Marker) -> IndirectMode {
        for arg in &marker.args {
            let arg = arg.trim();
            if let Some(value) = arg.strip_prefix("indirect=") {
                let value = value.trim();
                if value == "True" || value == "true" {
                    return IndirectMode::All;
                } else if value == "False" || value == "false" {
                    return IndirectMode::None;
                } else if value.starts_with('[') && value.ends_with(']') {
                    // Parse list of parameter names
                    let inner = &value[1..value.len() - 1];
                    let names: Vec<String> = inner
                        .split(',')
                        .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\'').to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    if !names.is_empty() {
                        return IndirectMode::Specific(names);
                    }
                }
            }
        }
        IndirectMode::None
    }

    /// Parse the values list from a parametrize decorator
    #[allow(clippy::type_complexity)]
    fn parse_values_list(
        &self,
        values_str: &str,
        num_args: usize,
    ) -> (Vec<Vec<String>>, Vec<Option<String>>, Vec<Vec<String>>) {
        let mut values = Vec::new();
        let mut ids = Vec::new();
        let mut marks = Vec::new();

        // Simple parsing - handle common cases
        let trimmed = values_str.trim();

        // Check if it's a list
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let inner = &trimmed[1..trimmed.len() - 1];
            let items = self.split_list_items(inner);

            for item in items.iter() {
                let item = item.trim();

                // Check for pytest.param wrapper
                if item.starts_with("pytest.param(") || item.starts_with("param(") {
                    let (vals, id, item_marks) = self.parse_pytest_param(item, num_args);
                    values.push(vals);
                    ids.push(id);
                    marks.push(item_marks);
                } else if num_args == 1 {
                    // Single parameter
                    values.push(vec![item.to_string()]);
                    ids.push(None);
                    marks.push(Vec::new());
                } else {
                    // Tuple of parameters
                    let tuple_vals = self.parse_tuple(item);
                    if tuple_vals.len() == num_args {
                        values.push(tuple_vals);
                        ids.push(None);
                        marks.push(Vec::new());
                    }
                }
            }
        }

        // Generate default IDs for items without custom IDs
        for (idx, id) in ids.iter_mut().enumerate() {
            if id.is_none() {
                *id = Some(idx.to_string());
            }
        }

        (values, ids, marks)
    }

    /// Split list items handling nested brackets and parentheses
    fn split_list_items(&self, s: &str) -> Vec<String> {
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

        let trimmed = current.trim().to_string();
        if !trimmed.is_empty() {
            items.push(trimmed);
        }

        items
    }

    /// Parse a pytest.param(...) wrapper
    fn parse_pytest_param(
        &self,
        s: &str,
        num_args: usize,
    ) -> (Vec<String>, Option<String>, Vec<String>) {
        let mut values = Vec::new();
        let mut id = None;
        let mut marks = Vec::new();

        // Extract content between parentheses
        let start = s.find('(').unwrap_or(0) + 1;
        let end = s.rfind(')').unwrap_or(s.len());
        let content = &s[start..end];

        let parts = self.split_list_items(content);

        // First parts are the values
        for (idx, part) in parts.iter().enumerate() {
            let part = part.trim();

            if let Some(stripped) = part.strip_prefix("id=") {
                // Custom ID
                let id_val = stripped.trim_matches(|c| c == '"' || c == '\'');
                id = Some(id_val.to_string());
            } else if let Some(marks_str) = part.strip_prefix("marks=") {
                // Marks
                marks = self.parse_marks(marks_str);
            } else if idx < num_args {
                values.push(part.to_string());
            } else if num_args == 1 && idx == 0 {
                // Single value case
                values.push(part.to_string());
            }
        }

        // If we got a tuple as first arg, expand it
        if values.len() == 1 && num_args > 1 {
            let tuple_vals = self.parse_tuple(&values[0]);
            if tuple_vals.len() == num_args {
                values = tuple_vals;
            }
        }

        (values, id, marks)
    }

    /// Parse marks from a marks= argument
    fn parse_marks(&self, s: &str) -> Vec<String> {
        let mut marks = Vec::new();
        let trimmed = s.trim();

        // Handle single mark or list of marks
        if trimmed.starts_with('[') {
            let inner = &trimmed[1..trimmed.len().saturating_sub(1)];
            for mark in self.split_list_items(inner) {
                marks.push(self.extract_mark_name(&mark));
            }
        } else {
            marks.push(self.extract_mark_name(trimmed));
        }

        marks
    }

    /// Extract mark name from pytest.mark.xxx or mark.xxx
    fn extract_mark_name(&self, s: &str) -> String {
        let s = s.trim();
        if let Some(stripped) = s.strip_prefix("pytest.mark.") {
            stripped.split('(').next().unwrap_or(s).to_string()
        } else if let Some(stripped) = s.strip_prefix("mark.") {
            stripped.split('(').next().unwrap_or(s).to_string()
        } else {
            s.split('(').next().unwrap_or(s).to_string()
        }
    }

    /// Parse a tuple like (1, 2) or (1, "hello")
    fn parse_tuple(&self, s: &str) -> Vec<String> {
        let trimmed = s.trim();

        // Handle tuple with or without parentheses
        let inner = if trimmed.starts_with('(') && trimmed.ends_with(')') {
            &trimmed[1..trimmed.len() - 1]
        } else {
            trimmed
        };

        self.split_list_items(inner)
    }

    /// Generate cartesian product of all parameter sets
    fn generate_cartesian_product(
        &self,
        test: &TestCase,
        param_sets: &[ParameterSet],
    ) -> Vec<ExpandedTest> {
        if param_sets.is_empty() {
            return vec![];
        }

        // Collect all indirect params from all parameter sets
        let mut all_indirect_params: Vec<String> = Vec::new();
        for param_set in param_sets {
            for argname in &param_set.argnames {
                if param_set.indirect.is_indirect(argname) && !all_indirect_params.contains(argname) {
                    all_indirect_params.push(argname.clone());
                }
            }
        }

        // Start with first parameter set
        #[allow(clippy::type_complexity)]
        let mut expanded: Vec<(
            HashMap<String, String>,
            Vec<String>,
            bool,
            Option<String>,
        )> = param_sets[0]
            .values
            .iter()
            .enumerate()
            .map(|(idx, vals)| {
                let mut params = HashMap::new();
                for (name, val) in param_sets[0].argnames.iter().zip(vals.iter()) {
                    params.insert(name.clone(), val.clone());
                }
                let id_parts =
                    vec![param_sets[0].ids[idx].clone().unwrap_or_else(|| idx.to_string())];
                let is_xfail = param_sets[0]
                    .marks
                    .get(idx)
                    .map(|m| m.iter().any(|mark| mark == "xfail"))
                    .unwrap_or(false);
                let skip_reason = param_sets[0]
                    .marks
                    .get(idx)
                    .and_then(|m| m.iter().find(|mark| mark.starts_with("skip")))
                    .map(|_| "marked to skip".to_string());
                (params, id_parts, is_xfail, skip_reason)
            })
            .collect();

        // Multiply with remaining parameter sets
        for param_set in param_sets.iter().skip(1) {
            let mut new_expanded = Vec::new();

            for (existing_params, existing_ids, existing_xfail, existing_skip) in &expanded {
                for (idx, vals) in param_set.values.iter().enumerate() {
                    let mut params = existing_params.clone();
                    for (name, val) in param_set.argnames.iter().zip(vals.iter()) {
                        params.insert(name.clone(), val.clone());
                    }

                    let mut id_parts = existing_ids.clone();
                    id_parts.push(param_set.ids[idx].clone().unwrap_or_else(|| idx.to_string()));

                    let is_xfail = *existing_xfail
                        || param_set
                            .marks
                            .get(idx)
                            .map(|m| m.iter().any(|mark| mark == "xfail"))
                            .unwrap_or(false);

                    let skip_reason = existing_skip.clone().or_else(|| {
                        param_set
                            .marks
                            .get(idx)
                            .and_then(|m| m.iter().find(|mark| mark.starts_with("skip")))
                            .map(|_| "marked to skip".to_string())
                    });

                    new_expanded.push((params, id_parts, is_xfail, skip_reason));
                }
            }

            expanded = new_expanded;
        }

        // Convert to ExpandedTest
        expanded
            .into_iter()
            .map(|(params, id_parts, is_xfail, skip_reason)| ExpandedTest {
                base: test.clone(),
                param_values: params,
                indirect_params: all_indirect_params.clone(),
                id_suffix: id_parts.join("-"),
                expected_failure: is_xfail,
                skip_reason,
            })
            .collect()
    }

    /// Expand multiple tests
    pub fn expand_all(&self, tests: &[TestCase]) -> Vec<ExpandedTest> {
        tests.iter().flat_map(|t| self.expand(t)).collect()
    }
}

impl Default for ParametrizeExpander {
    fn default() -> Self {
        Self::new()
    }
}
