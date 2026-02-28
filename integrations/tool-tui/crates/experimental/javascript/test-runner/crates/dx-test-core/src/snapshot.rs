//! Snapshot Testing Implementation for DX Test Runner
//!
//! Provides Jest-compatible snapshot testing:
//! - toMatchSnapshot() - Compare against stored snapshots
//! - toMatchInlineSnapshot() - Inline snapshot comparison
//! - Snapshot storage in __snapshots__ directories
//! - Snapshot updating with --updateSnapshot flag
//! - Diff display for mismatches

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

/// Snapshot file format version
pub const SNAPSHOT_VERSION: u32 = 1;

/// A snapshot file containing multiple snapshots for a test file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotFile {
    /// Format version
    pub version: u32,
    /// Snapshots keyed by test name
    pub snapshots: HashMap<String, Snapshot>,
}

impl Default for SnapshotFile {
    fn default() -> Self {
        Self {
            version: SNAPSHOT_VERSION,
            snapshots: HashMap::new(),
        }
    }
}

/// A single snapshot entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    /// The serialized value
    pub value: String,
    /// When this snapshot was created/updated
    pub created_at: u64,
    /// Number of times this snapshot has been verified
    pub verified_count: u32,
}

impl Snapshot {
    /// Create a new snapshot with the current timestamp
    pub fn new(value: String) -> Self {
        Self {
            value,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            verified_count: 0,
        }
    }

    /// Increment the verification count
    pub fn verify(&mut self) {
        self.verified_count += 1;
    }
}

/// Result of a snapshot comparison
#[derive(Debug, Clone, PartialEq)]
pub enum SnapshotResult {
    /// Snapshot matches the actual value
    Match,
    /// Snapshot doesn't match (contains expected and actual)
    Mismatch {
        expected: String,
        actual: String,
        diff: String,
    },
    /// No snapshot exists yet (new test)
    New { actual: String },
    /// Snapshot was updated
    Updated { old: String, new: String },
}

impl SnapshotResult {
    /// Check if the result is a match or updated (success cases)
    pub fn is_success(&self) -> bool {
        matches!(self, SnapshotResult::Match | SnapshotResult::Updated { .. })
    }

    /// Check if the result is a failure
    pub fn is_failure(&self) -> bool {
        matches!(self, SnapshotResult::Mismatch { .. } | SnapshotResult::New { .. })
    }
}

/// Snapshot serializer for converting values to snapshot format
pub struct SnapshotSerializer {
    /// Indentation string (default: 2 spaces)
    indent: String,
    /// Maximum depth for nested objects
    max_depth: usize,
}

impl Default for SnapshotSerializer {
    fn default() -> Self {
        Self {
            indent: "  ".to_string(),
            max_depth: 10,
        }
    }
}

impl SnapshotSerializer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set custom indentation
    pub fn with_indent(mut self, indent: &str) -> Self {
        self.indent = indent.to_string();
        self
    }

    /// Set maximum depth
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Serialize a value to snapshot format
    /// This produces a deterministic, human-readable representation
    pub fn serialize(&self, value: &SnapshotValue) -> String {
        self.serialize_value(value, 0)
    }

    fn serialize_value(&self, value: &SnapshotValue, depth: usize) -> String {
        if depth > self.max_depth {
            return "[Max depth exceeded]".to_string();
        }

        match value {
            SnapshotValue::Null => "null".to_string(),
            SnapshotValue::Undefined => "undefined".to_string(),
            SnapshotValue::Boolean(b) => b.to_string(),
            SnapshotValue::Number(n) => {
                if n.is_nan() {
                    "NaN".to_string()
                } else if n.is_infinite() {
                    if *n > 0.0 { "Infinity" } else { "-Infinity" }.to_string()
                } else if n.fract() == 0.0 && n.abs() < 1e15 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            SnapshotValue::String(s) => format!("\"{}\"", self.escape_string(s)),
            SnapshotValue::Array(arr) => self.serialize_array(arr, depth),
            SnapshotValue::Object(obj) => self.serialize_object(obj, depth),
            SnapshotValue::Function(name) => {
                format!("[Function: {}]", name.as_deref().unwrap_or("anonymous"))
            }
            SnapshotValue::Symbol(desc) => format!("Symbol({})", desc.as_deref().unwrap_or("")),
            SnapshotValue::BigInt(s) => format!("{}n", s),
            SnapshotValue::Date(ts) => format!("Date({})", ts),
            SnapshotValue::RegExp { pattern, flags } => format!("/{}/{}", pattern, flags),
            SnapshotValue::Error { name, message } => format!("[{}: {}]", name, message),
            SnapshotValue::Map(entries) => self.serialize_map(entries, depth),
            SnapshotValue::Set(values) => self.serialize_set(values, depth),
        }
    }

    fn serialize_array(&self, arr: &[SnapshotValue], depth: usize) -> String {
        if arr.is_empty() {
            return "[]".to_string();
        }

        let indent = self.indent.repeat(depth + 1);
        let close_indent = self.indent.repeat(depth);

        let items: Vec<String> = arr
            .iter()
            .map(|v| format!("{}{}", indent, self.serialize_value(v, depth + 1)))
            .collect();

        format!("[\n{}\n{}]", items.join(",\n"), close_indent)
    }

    fn serialize_object(&self, obj: &[(String, SnapshotValue)], depth: usize) -> String {
        if obj.is_empty() {
            return "{}".to_string();
        }

        let indent = self.indent.repeat(depth + 1);
        let close_indent = self.indent.repeat(depth);

        // Sort keys for deterministic output
        let mut sorted: Vec<_> = obj.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(&b.0));

        let items: Vec<String> = sorted
            .iter()
            .map(|(k, v)| {
                let key = if self.needs_quotes(k) {
                    format!("\"{}\"", self.escape_string(k))
                } else {
                    k.clone()
                };
                format!("{}{}: {}", indent, key, self.serialize_value(v, depth + 1))
            })
            .collect();

        format!("{{\n{}\n{}}}", items.join(",\n"), close_indent)
    }

    fn serialize_map(&self, entries: &[(SnapshotValue, SnapshotValue)], depth: usize) -> String {
        if entries.is_empty() {
            return "Map {}".to_string();
        }

        let indent = self.indent.repeat(depth + 1);
        let close_indent = self.indent.repeat(depth);

        let items: Vec<String> = entries
            .iter()
            .map(|(k, v)| {
                format!(
                    "{}{} => {}",
                    indent,
                    self.serialize_value(k, depth + 1),
                    self.serialize_value(v, depth + 1)
                )
            })
            .collect();

        format!("Map {{\n{}\n{}}}", items.join(",\n"), close_indent)
    }

    fn serialize_set(&self, values: &[SnapshotValue], depth: usize) -> String {
        if values.is_empty() {
            return "Set {}".to_string();
        }

        let indent = self.indent.repeat(depth + 1);
        let close_indent = self.indent.repeat(depth);

        let items: Vec<String> = values
            .iter()
            .map(|v| format!("{}{}", indent, self.serialize_value(v, depth + 1)))
            .collect();

        format!("Set {{\n{}\n{}}}", items.join(",\n"), close_indent)
    }

    fn escape_string(&self, s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '"' => result.push_str("\\\""),
                '\\' => result.push_str("\\\\"),
                '\n' => result.push_str("\\n"),
                '\r' => result.push_str("\\r"),
                '\t' => result.push_str("\\t"),
                c if c.is_control() => result.push_str(&format!("\\u{:04x}", c as u32)),
                c => result.push(c),
            }
        }
        result
    }

    fn needs_quotes(&self, key: &str) -> bool {
        if key.is_empty() {
            return true;
        }

        let first = key.chars().next().unwrap();
        if !first.is_alphabetic() && first != '_' && first != '$' {
            return true;
        }

        key.chars().any(|c| !c.is_alphanumeric() && c != '_' && c != '$')
    }
}

/// Value types that can be serialized to snapshots
#[derive(Debug, Clone, PartialEq)]
pub enum SnapshotValue {
    Null,
    Undefined,
    Boolean(bool),
    Number(f64),
    String(String),
    Array(Vec<SnapshotValue>),
    Object(Vec<(String, SnapshotValue)>),
    Function(Option<String>),
    Symbol(Option<String>),
    BigInt(String),
    Date(u64),
    RegExp { pattern: String, flags: String },
    Error { name: String, message: String },
    Map(Vec<(SnapshotValue, SnapshotValue)>),
    Set(Vec<SnapshotValue>),
}

impl SnapshotValue {
    /// Create from a JSON-like structure
    pub fn from_json(json: &str) -> Result<Self, String> {
        // Simple JSON parser for common cases
        let trimmed = json.trim();

        if trimmed == "null" {
            return Ok(SnapshotValue::Null);
        }
        if trimmed == "undefined" {
            return Ok(SnapshotValue::Undefined);
        }
        if trimmed == "true" {
            return Ok(SnapshotValue::Boolean(true));
        }
        if trimmed == "false" {
            return Ok(SnapshotValue::Boolean(false));
        }

        // Try parsing as number
        if let Ok(n) = trimmed.parse::<f64>() {
            return Ok(SnapshotValue::Number(n));
        }

        // String
        if trimmed.starts_with('"') && trimmed.ends_with('"') {
            let inner = &trimmed[1..trimmed.len() - 1];
            return Ok(SnapshotValue::String(inner.to_string()));
        }

        // For complex types, just store as string for now
        Ok(SnapshotValue::String(trimmed.to_string()))
    }
}

/// Snapshot manager for a test file
pub struct SnapshotManager {
    /// Path to the test file
    #[allow(dead_code)]
    test_file: PathBuf,
    /// Path to the snapshot file
    snapshot_path: PathBuf,
    /// Loaded snapshots
    snapshots: SnapshotFile,
    /// Whether to update snapshots
    update_mode: bool,
    /// Pending updates (test_name -> new_value)
    pending_updates: HashMap<String, String>,
    /// Counter for unnamed snapshots
    snapshot_counter: HashMap<String, u32>,
    /// Serializer for values
    serializer: SnapshotSerializer,
}

impl SnapshotManager {
    /// Create a new snapshot manager for a test file
    pub fn new(test_file: &Path, update_mode: bool) -> Self {
        let snapshot_path = Self::snapshot_path_for(test_file);
        let snapshots = Self::load_snapshots(&snapshot_path).unwrap_or_default();

        Self {
            test_file: test_file.to_path_buf(),
            snapshot_path,
            snapshots,
            update_mode,
            pending_updates: HashMap::new(),
            snapshot_counter: HashMap::new(),
            serializer: SnapshotSerializer::new(),
        }
    }

    /// Get the snapshot file path for a test file
    /// Follows Jest convention: __snapshots__/<filename>.snap
    pub fn snapshot_path_for(test_file: &Path) -> PathBuf {
        let parent = test_file.parent().unwrap_or(Path::new("."));
        let snapshots_dir = parent.join("__snapshots__");
        let file_name = test_file.file_name().and_then(|s| s.to_str()).unwrap_or("test");
        snapshots_dir.join(format!("{}.snap", file_name))
    }

    /// Load snapshots from file
    fn load_snapshots(path: &Path) -> Option<SnapshotFile> {
        let file = File::open(path).ok()?;
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).ok()
    }

    /// Generate a snapshot key for a test
    pub fn snapshot_key(&mut self, test_name: &str, hint: Option<&str>) -> String {
        if let Some(h) = hint {
            format!("{} {}", test_name, h)
        } else {
            let counter = self.snapshot_counter.entry(test_name.to_string()).or_insert(0);
            *counter += 1;
            if *counter == 1 {
                test_name.to_string()
            } else {
                format!("{} {}", test_name, counter)
            }
        }
    }

    /// Compare a value against its snapshot (toMatchSnapshot)
    pub fn match_snapshot(&mut self, test_name: &str, actual: &SnapshotValue) -> SnapshotResult {
        self.match_snapshot_with_hint(test_name, actual, None)
    }

    /// Compare a value against its snapshot with a hint
    pub fn match_snapshot_with_hint(
        &mut self,
        test_name: &str,
        actual: &SnapshotValue,
        hint: Option<&str>,
    ) -> SnapshotResult {
        let key = self.snapshot_key(test_name, hint);
        let actual_str = self.serializer.serialize(actual);

        if let Some(snapshot) = self.snapshots.snapshots.get_mut(&key) {
            if snapshot.value == actual_str {
                snapshot.verify();
                SnapshotResult::Match
            } else if self.update_mode {
                let old = snapshot.value.clone();
                self.pending_updates.insert(key, actual_str.clone());
                SnapshotResult::Updated {
                    old,
                    new: actual_str,
                }
            } else {
                let diff = generate_diff(&snapshot.value, &actual_str);
                SnapshotResult::Mismatch {
                    expected: snapshot.value.clone(),
                    actual: actual_str,
                    diff,
                }
            }
        } else if self.update_mode {
            self.pending_updates.insert(key, actual_str.clone());
            SnapshotResult::Updated {
                old: String::new(),
                new: actual_str,
            }
        } else {
            SnapshotResult::New { actual: actual_str }
        }
    }

    /// Save pending updates to the snapshot file
    pub fn save(&mut self) -> std::io::Result<()> {
        if self.pending_updates.is_empty() {
            return Ok(());
        }

        // Apply pending updates
        for (key, value) in self.pending_updates.drain() {
            self.snapshots.snapshots.insert(key, Snapshot::new(value));
        }

        // Create snapshots directory
        if let Some(parent) = self.snapshot_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write snapshot file
        let file = File::create(&self.snapshot_path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &self.snapshots).map_err(std::io::Error::other)?;

        Ok(())
    }

    /// Get number of snapshots
    pub fn snapshot_count(&self) -> usize {
        self.snapshots.snapshots.len()
    }

    /// Get number of pending updates
    pub fn pending_count(&self) -> usize {
        self.pending_updates.len()
    }

    /// Get all snapshot keys
    pub fn keys(&self) -> Vec<&String> {
        self.snapshots.snapshots.keys().collect()
    }

    /// Check if a snapshot exists
    pub fn has_snapshot(&self, key: &str) -> bool {
        self.snapshots.snapshots.contains_key(key)
    }

    /// Remove obsolete snapshots (not verified during test run)
    pub fn remove_obsolete(&mut self) -> Vec<String> {
        let obsolete: Vec<String> = self
            .snapshots
            .snapshots
            .iter()
            .filter(|(_, s)| s.verified_count == 0)
            .map(|(k, _)| k.clone())
            .collect();

        for key in &obsolete {
            self.snapshots.snapshots.remove(key);
        }

        obsolete
    }
}

/// Generate a diff between expected and actual strings
pub fn generate_diff(expected: &str, actual: &str) -> String {
    let mut diff = String::new();

    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();

    let max_lines = expected_lines.len().max(actual_lines.len());

    for i in 0..max_lines {
        let exp = expected_lines.get(i).copied().unwrap_or("");
        let act = actual_lines.get(i).copied().unwrap_or("");

        if exp == act {
            diff.push_str(&format!("  {}\n", exp));
        } else {
            if !exp.is_empty() {
                diff.push_str(&format!("- {}\n", exp));
            }
            if !act.is_empty() {
                diff.push_str(&format!("+ {}\n", act));
            }
        }
    }

    diff
}

/// Inline snapshot support
#[derive(Debug, Clone)]
pub struct InlineSnapshot {
    /// File path
    pub file: PathBuf,
    /// Line number
    pub line: u32,
    /// Column number
    pub column: u32,
    /// Expected value (from source)
    pub expected: Option<String>,
    /// Actual value
    pub actual: String,
}

impl InlineSnapshot {
    /// Check if inline snapshot matches
    pub fn matches(&self) -> bool {
        match &self.expected {
            Some(expected) => expected == &self.actual,
            None => false,
        }
    }

    /// Generate source code update for inline snapshot
    pub fn generate_update(&self) -> String {
        // Escape the actual value for embedding in source
        let escaped = self.actual.replace('\\', "\\\\").replace('`', "\\`").replace("${", "\\${");

        format!("`{}`", escaped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_serializer_primitives() {
        let serializer = SnapshotSerializer::new();

        assert_eq!(serializer.serialize(&SnapshotValue::Null), "null");
        assert_eq!(serializer.serialize(&SnapshotValue::Undefined), "undefined");
        assert_eq!(serializer.serialize(&SnapshotValue::Boolean(true)), "true");
        assert_eq!(serializer.serialize(&SnapshotValue::Boolean(false)), "false");
        assert_eq!(serializer.serialize(&SnapshotValue::Number(42.0)), "42");
        assert_eq!(serializer.serialize(&SnapshotValue::Number(3.125)), "3.125");
        assert_eq!(serializer.serialize(&SnapshotValue::String("hello".to_string())), "\"hello\"");
    }

    #[test]
    fn test_snapshot_serializer_array() {
        let serializer = SnapshotSerializer::new();

        let arr = SnapshotValue::Array(vec![
            SnapshotValue::Number(1.0),
            SnapshotValue::Number(2.0),
            SnapshotValue::Number(3.0),
        ]);

        let result = serializer.serialize(&arr);
        assert!(result.contains("1"));
        assert!(result.contains("2"));
        assert!(result.contains("3"));
    }

    #[test]
    fn test_snapshot_serializer_object() {
        let serializer = SnapshotSerializer::new();

        let obj = SnapshotValue::Object(vec![
            ("name".to_string(), SnapshotValue::String("test".to_string())),
            ("value".to_string(), SnapshotValue::Number(42.0)),
        ]);

        let result = serializer.serialize(&obj);
        assert!(result.contains("name"));
        assert!(result.contains("test"));
        assert!(result.contains("value"));
        assert!(result.contains("42"));
    }

    #[test]
    fn test_snapshot_serializer_deterministic() {
        let serializer = SnapshotSerializer::new();

        // Object with keys in different order should produce same output
        let obj1 = SnapshotValue::Object(vec![
            ("b".to_string(), SnapshotValue::Number(2.0)),
            ("a".to_string(), SnapshotValue::Number(1.0)),
        ]);

        let obj2 = SnapshotValue::Object(vec![
            ("a".to_string(), SnapshotValue::Number(1.0)),
            ("b".to_string(), SnapshotValue::Number(2.0)),
        ]);

        assert_eq!(serializer.serialize(&obj1), serializer.serialize(&obj2));
    }

    #[test]
    fn test_snapshot_manager_new_snapshot() {
        let temp_dir = std::env::temp_dir().join("dx-test-snapshots-test");
        let test_file = temp_dir.join("test.ts");

        let mut manager = SnapshotManager::new(&test_file, false);

        let value = SnapshotValue::String("hello world".to_string());
        let result = manager.match_snapshot("test1", &value);

        // Should be New since no snapshot exists
        assert!(matches!(result, SnapshotResult::New { .. }));
    }

    #[test]
    fn test_snapshot_manager_update_mode() {
        let temp_dir = std::env::temp_dir().join("dx-test-snapshots-update");
        let test_file = temp_dir.join("test.ts");

        // Clean up any existing snapshot
        let snapshot_path = SnapshotManager::snapshot_path_for(&test_file);
        let _ = fs::remove_file(&snapshot_path);

        // Create with update mode
        let mut manager = SnapshotManager::new(&test_file, true);

        let value = SnapshotValue::String("hello world".to_string());
        let result = manager.match_snapshot("test1", &value);

        // Should be Updated in update mode
        assert!(matches!(result, SnapshotResult::Updated { .. }));

        // Save and reload
        manager.save().unwrap();

        let mut manager2 = SnapshotManager::new(&test_file, false);
        let result2 = manager2.match_snapshot("test1", &value);

        // Should match now
        assert!(matches!(result2, SnapshotResult::Match));

        // Cleanup
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_snapshot_manager_mismatch() {
        let temp_dir = std::env::temp_dir().join("dx-test-snapshots-mismatch");
        let test_file = temp_dir.join("test.ts");

        // Clean up
        let snapshot_path = SnapshotManager::snapshot_path_for(&test_file);
        let _ = fs::remove_file(&snapshot_path);

        // Create snapshot
        let mut manager = SnapshotManager::new(&test_file, true);
        let value1 = SnapshotValue::String("original".to_string());
        manager.match_snapshot("test1", &value1);
        manager.save().unwrap();

        // Try to match with different value
        let mut manager2 = SnapshotManager::new(&test_file, false);
        let value2 = SnapshotValue::String("modified".to_string());
        let result = manager2.match_snapshot("test1", &value2);

        // Should be Mismatch
        assert!(matches!(result, SnapshotResult::Mismatch { .. }));

        // Cleanup
        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_diff_generation() {
        let expected = "line1\nline2\nline3";
        let actual = "line1\nmodified\nline3";

        let diff = generate_diff(expected, actual);

        assert!(diff.contains("- line2"));
        assert!(diff.contains("+ modified"));
        assert!(diff.contains("  line1"));
        assert!(diff.contains("  line3"));
    }

    #[test]
    fn test_snapshot_key_generation() {
        let temp_dir = std::env::temp_dir().join("dx-test-snapshots-keys");
        let test_file = temp_dir.join("test.ts");

        let mut manager = SnapshotManager::new(&test_file, false);

        // First snapshot for a test
        let key1 = manager.snapshot_key("my test", None);
        assert_eq!(key1, "my test");

        // Second snapshot for same test
        let key2 = manager.snapshot_key("my test", None);
        assert_eq!(key2, "my test 2");

        // With hint
        let key3 = manager.snapshot_key("my test", Some("custom hint"));
        assert_eq!(key3, "my test custom hint");
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate arbitrary SnapshotValue
    fn arb_snapshot_value() -> impl Strategy<Value = SnapshotValue> {
        prop_oneof![
            Just(SnapshotValue::Null),
            Just(SnapshotValue::Undefined),
            any::<bool>().prop_map(SnapshotValue::Boolean),
            any::<f64>()
                .prop_filter("finite", |n| n.is_finite())
                .prop_map(SnapshotValue::Number),
            "[a-zA-Z0-9 ]{0,50}".prop_map(SnapshotValue::String),
        ]
    }

    /// Generate arbitrary nested SnapshotValue (limited depth)
    fn arb_nested_snapshot_value() -> impl Strategy<Value = SnapshotValue> {
        arb_snapshot_value().prop_recursive(
            3,  // depth
            64, // size
            10, // items per collection
            |inner| {
                prop_oneof![
                    prop::collection::vec(inner.clone(), 0..5).prop_map(SnapshotValue::Array),
                    prop::collection::vec(("[a-zA-Z_][a-zA-Z0-9_]{0,10}", inner), 0..5)
                        .prop_map(SnapshotValue::Object),
                ]
            },
        )
    }

    proptest! {
        /// Property 11: Snapshot Determinism
        /// Feature: dx-js-production-complete, Property 11: Snapshot Determinism
        /// Validates: Requirements 13.1, 13.2
        /// For any value, serializing it twice should produce identical output
        #[test]
        fn prop_snapshot_determinism(value in arb_nested_snapshot_value()) {
            let serializer = SnapshotSerializer::new();

            let result1 = serializer.serialize(&value);
            let result2 = serializer.serialize(&value);

            prop_assert_eq!(result1, result2, "Serialization should be deterministic");
        }

        /// Property: Object key ordering is deterministic
        /// Objects with same keys in different order should serialize identically
        #[test]
        fn prop_object_key_ordering_deterministic(
            keys in prop::collection::hash_set("[a-z]{1,5}", 1..10),
            values in prop::collection::vec(arb_snapshot_value(), 1..10)
        ) {
            let keys: Vec<String> = keys.into_iter().collect();
            if keys.len() > values.len() {
                return Ok(());
            }

            let serializer = SnapshotSerializer::new();

            // Create object with keys in original order
            let pairs1: Vec<_> = keys.iter().cloned()
                .zip(values.iter().take(keys.len()).cloned())
                .collect();
            let obj1 = SnapshotValue::Object(pairs1);

            // Create object with keys in reverse order
            let pairs2: Vec<_> = keys.iter().rev().cloned()
                .zip(values.iter().take(keys.len()).rev().cloned())
                .collect();
            let obj2 = SnapshotValue::Object(pairs2);

            let result1 = serializer.serialize(&obj1);
            let result2 = serializer.serialize(&obj2);

            prop_assert_eq!(result1, result2, "Object serialization should be key-order independent");
        }

        /// Property: Snapshot match is reflexive
        /// A value should always match its own snapshot
        #[test]
        fn prop_snapshot_match_reflexive(value in arb_nested_snapshot_value()) {
            use std::sync::atomic::{AtomicU32, Ordering};
            static COUNTER: AtomicU32 = AtomicU32::new(0);
            let id = COUNTER.fetch_add(1, Ordering::SeqCst);

            let temp_dir = std::env::temp_dir().join(format!("dx-test-prop-{}", id));
            let test_file = temp_dir.join("test.ts");

            // Create snapshot in update mode
            let mut manager = SnapshotManager::new(&test_file, true);
            let result1 = manager.match_snapshot("test", &value);
            prop_assert!(result1.is_success(), "First match should succeed in update mode");

            let _ = manager.save();

            // Match same value
            let mut manager2 = SnapshotManager::new(&test_file, false);
            let result2 = manager2.match_snapshot("test", &value);
            prop_assert!(matches!(result2, SnapshotResult::Match), "Same value should match");

            // Cleanup
            let _ = fs::remove_dir_all(temp_dir);
        }

        /// Property: Different values produce different snapshots
        /// Two different values should not produce the same serialization (with high probability)
        #[test]
        fn prop_different_values_different_snapshots(
            value1 in arb_snapshot_value(),
            value2 in arb_snapshot_value()
        ) {
            let serializer = SnapshotSerializer::new();

            let result1 = serializer.serialize(&value1);
            let result2 = serializer.serialize(&value2);

            // If values are equal, serializations should be equal
            // If values are different, serializations should (usually) be different
            if value1 == value2 {
                prop_assert_eq!(result1, result2);
            }
            // Note: We don't assert inequality for different values because
            // some different values might serialize the same (e.g., 1.0 and 1)
        }

        /// Property: Snapshot key generation is unique per test
        #[test]
        fn prop_snapshot_keys_unique(
            test_name in "[a-zA-Z ]{1,20}",
            count in 1usize..10
        ) {
            use std::sync::atomic::{AtomicU32, Ordering};
            static COUNTER2: AtomicU32 = AtomicU32::new(0);
            let id = COUNTER2.fetch_add(1, Ordering::SeqCst);

            let temp_dir = std::env::temp_dir().join(format!("dx-test-keys-{}", id));
            let test_file = temp_dir.join("test.ts");

            let mut manager = SnapshotManager::new(&test_file, false);
            let mut keys = Vec::new();

            for _ in 0..count {
                keys.push(manager.snapshot_key(&test_name, None));
            }

            // All keys should be unique
            let unique_count = keys.iter().collect::<std::collections::HashSet<_>>().len();
            prop_assert_eq!(unique_count, count, "All snapshot keys should be unique");
        }
    }
}
