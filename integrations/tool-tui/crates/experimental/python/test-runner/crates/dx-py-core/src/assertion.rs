//! Assertion introspection for detailed test failure messages
//!
//! This module provides types and utilities for capturing assertion values,
//! computing diffs, and generating detailed failure messages.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a captured value from an assertion
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AssertionValue {
    /// A string value
    String(String),
    /// An integer value
    Integer(i64),
    /// A floating point value
    Float(f64),
    /// A boolean value
    Bool(bool),
    /// A list/array of values
    List(Vec<AssertionValue>),
    /// A dictionary/map of values
    Dict(Vec<(String, AssertionValue)>),
    /// A set of values
    Set(Vec<AssertionValue>),
    /// None/null value
    None,
    /// A custom object representation
    Object { type_name: String, repr: String },
}

impl fmt::Display for AssertionValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssertionValue::String(s) => write!(f, "'{}'", s),
            AssertionValue::Integer(i) => write!(f, "{}", i),
            AssertionValue::Float(fl) => write!(f, "{}", fl),
            AssertionValue::Bool(b) => write!(f, "{}", if *b { "True" } else { "False" }),
            AssertionValue::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            AssertionValue::Dict(items) => {
                write!(f, "{{")?;
                for (i, (key, value)) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "'{}': {}", key, value)?;
                }
                write!(f, "}}")
            }
            AssertionValue::Set(items) => {
                write!(f, "{{")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "}}")
            }
            AssertionValue::None => write!(f, "None"),
            AssertionValue::Object { type_name, repr } => write!(f, "<{}: {}>", type_name, repr),
        }
    }
}

/// The type of comparison in an assertion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparisonOp {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    In,
    NotIn,
    Is,
    IsNot,
}

impl fmt::Display for ComparisonOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComparisonOp::Equal => write!(f, "=="),
            ComparisonOp::NotEqual => write!(f, "!="),
            ComparisonOp::LessThan => write!(f, "<"),
            ComparisonOp::LessThanOrEqual => write!(f, "<="),
            ComparisonOp::GreaterThan => write!(f, ">"),
            ComparisonOp::GreaterThanOrEqual => write!(f, ">="),
            ComparisonOp::In => write!(f, "in"),
            ComparisonOp::NotIn => write!(f, "not in"),
            ComparisonOp::Is => write!(f, "is"),
            ComparisonOp::IsNot => write!(f, "is not"),
        }
    }
}

/// Captured assertion failure with introspection data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssertionFailure {
    /// The original assertion expression text
    pub expression: String,
    /// The left-hand side value
    pub left: AssertionValue,
    /// The right-hand side value (if applicable)
    pub right: Option<AssertionValue>,
    /// The comparison operator
    pub op: Option<ComparisonOp>,
    /// Custom message provided by the user
    pub message: Option<String>,
    /// File where the assertion occurred
    pub file: String,
    /// Line number of the assertion
    pub line: u32,
}

impl AssertionFailure {
    /// Create a new assertion failure for a simple assertion (assert x)
    pub fn simple(
        expression: impl Into<String>,
        value: AssertionValue,
        file: impl Into<String>,
        line: u32,
    ) -> Self {
        Self {
            expression: expression.into(),
            left: value,
            right: None,
            op: None,
            message: None,
            file: file.into(),
            line,
        }
    }

    /// Create a new assertion failure for a comparison (assert x == y)
    pub fn comparison(
        expression: impl Into<String>,
        left: AssertionValue,
        op: ComparisonOp,
        right: AssertionValue,
        file: impl Into<String>,
        line: u32,
    ) -> Self {
        Self {
            expression: expression.into(),
            left,
            right: Some(right),
            op: Some(op),
            message: None,
            file: file.into(),
            line,
        }
    }

    /// Add a custom message to the assertion failure
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Extract left value, returning a reference
    pub fn left_value(&self) -> &AssertionValue {
        &self.left
    }

    /// Extract right value if present
    pub fn right_value(&self) -> Option<&AssertionValue> {
        self.right.as_ref()
    }
}

/// Represents a difference between two values
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DiffSegment {
    /// Text that is the same in both
    Equal(String),
    /// Text that was deleted (only in left)
    Deleted(String),
    /// Text that was inserted (only in right)
    Inserted(String),
    /// Text that was changed
    Changed { from: String, to: String },
}

/// Result of diffing two strings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StringDiff {
    pub segments: Vec<DiffSegment>,
}

impl StringDiff {
    /// Compute a character-level diff between two strings
    pub fn compute(left: &str, right: &str) -> Self {
        if left == right {
            return Self {
                segments: vec![DiffSegment::Equal(left.to_string())],
            };
        }

        if left.is_empty() {
            return Self {
                segments: vec![DiffSegment::Inserted(right.to_string())],
            };
        }

        if right.is_empty() {
            return Self {
                segments: vec![DiffSegment::Deleted(left.to_string())],
            };
        }

        // Use a simple LCS-based diff algorithm
        let left_chars: Vec<char> = left.chars().collect();
        let right_chars: Vec<char> = right.chars().collect();

        let segments = Self::compute_lcs_diff(&left_chars, &right_chars);
        Self { segments }
    }

    /// Compute diff using LCS (Longest Common Subsequence) algorithm
    fn compute_lcs_diff(left: &[char], right: &[char]) -> Vec<DiffSegment> {
        let m = left.len();
        let n = right.len();

        // Build LCS table
        let mut dp = vec![vec![0usize; n + 1]; m + 1];
        for i in 1..=m {
            for j in 1..=n {
                if left[i - 1] == right[j - 1] {
                    dp[i][j] = dp[i - 1][j - 1] + 1;
                } else {
                    dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
                }
            }
        }

        // Backtrack to find the diff
        let mut segments = Vec::new();
        let mut i = m;
        let mut j = n;
        let mut equal_buf = String::new();
        let mut deleted_buf = String::new();
        let mut inserted_buf = String::new();

        while i > 0 || j > 0 {
            if i > 0 && j > 0 && left[i - 1] == right[j - 1] {
                // Flush any pending deletions/insertions
                Self::flush_buffers(&mut segments, &mut deleted_buf, &mut inserted_buf);
                equal_buf.insert(0, left[i - 1]);
                i -= 1;
                j -= 1;
            } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
                // Flush equal buffer
                Self::flush_equal(&mut segments, &mut equal_buf);
                inserted_buf.insert(0, right[j - 1]);
                j -= 1;
            } else if i > 0 {
                // Flush equal buffer
                Self::flush_equal(&mut segments, &mut equal_buf);
                deleted_buf.insert(0, left[i - 1]);
                i -= 1;
            }
        }

        // Flush remaining buffers
        Self::flush_equal(&mut segments, &mut equal_buf);
        Self::flush_buffers(&mut segments, &mut deleted_buf, &mut inserted_buf);

        // Reverse since we built it backwards
        segments.reverse();

        // Merge adjacent segments of the same type
        Self::merge_segments(segments)
    }

    fn flush_equal(segments: &mut Vec<DiffSegment>, buf: &mut String) {
        if !buf.is_empty() {
            segments.push(DiffSegment::Equal(std::mem::take(buf)));
        }
    }

    fn flush_buffers(segments: &mut Vec<DiffSegment>, deleted: &mut String, inserted: &mut String) {
        if !deleted.is_empty() && !inserted.is_empty() {
            segments.push(DiffSegment::Changed {
                from: std::mem::take(deleted),
                to: std::mem::take(inserted),
            });
        } else {
            if !deleted.is_empty() {
                segments.push(DiffSegment::Deleted(std::mem::take(deleted)));
            }
            if !inserted.is_empty() {
                segments.push(DiffSegment::Inserted(std::mem::take(inserted)));
            }
        }
    }

    fn merge_segments(segments: Vec<DiffSegment>) -> Vec<DiffSegment> {
        let mut merged = Vec::new();
        for seg in segments {
            if let Some(last) = merged.last_mut() {
                match (last, &seg) {
                    (DiffSegment::Equal(ref mut s1), DiffSegment::Equal(s2)) => {
                        s1.push_str(s2);
                        continue;
                    }
                    (DiffSegment::Deleted(ref mut s1), DiffSegment::Deleted(s2)) => {
                        s1.push_str(s2);
                        continue;
                    }
                    (DiffSegment::Inserted(ref mut s1), DiffSegment::Inserted(s2)) => {
                        s1.push_str(s2);
                        continue;
                    }
                    _ => {}
                }
            }
            merged.push(seg);
        }
        merged
    }

    /// Format the diff for display with ANSI colors
    pub fn format_colored(&self) -> String {
        let mut result = String::new();
        for segment in &self.segments {
            match segment {
                DiffSegment::Equal(s) => result.push_str(s),
                DiffSegment::Deleted(s) => {
                    result.push_str("\x1b[31m"); // Red
                    result.push_str("[-");
                    result.push_str(s);
                    result.push_str("-]");
                    result.push_str("\x1b[0m");
                }
                DiffSegment::Inserted(s) => {
                    result.push_str("\x1b[32m"); // Green
                    result.push_str("[+");
                    result.push_str(s);
                    result.push_str("+]");
                    result.push_str("\x1b[0m");
                }
                DiffSegment::Changed { from, to } => {
                    result.push_str("\x1b[31m"); // Red
                    result.push_str("[-");
                    result.push_str(from);
                    result.push_str("-]");
                    result.push_str("\x1b[0m");
                    result.push_str("\x1b[32m"); // Green
                    result.push_str("[+");
                    result.push_str(to);
                    result.push_str("+]");
                    result.push_str("\x1b[0m");
                }
            }
        }
        result
    }

    /// Format the diff for display without colors
    pub fn format_plain(&self) -> String {
        let mut result = String::new();
        for segment in &self.segments {
            match segment {
                DiffSegment::Equal(s) => result.push_str(s),
                DiffSegment::Deleted(s) => {
                    result.push_str("[-");
                    result.push_str(s);
                    result.push_str("-]");
                }
                DiffSegment::Inserted(s) => {
                    result.push_str("[+");
                    result.push_str(s);
                    result.push_str("+]");
                }
                DiffSegment::Changed { from, to } => {
                    result.push_str("[-");
                    result.push_str(from);
                    result.push_str("-][+");
                    result.push_str(to);
                    result.push_str("+]");
                }
            }
        }
        result
    }
}

/// Represents a difference in a collection element
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CollectionDiffItem {
    /// Element exists in both collections at the same position
    Same { index: usize, value: AssertionValue },
    /// Element was added (only in right)
    Added { index: usize, value: AssertionValue },
    /// Element was removed (only in left)
    Removed { index: usize, value: AssertionValue },
    /// Element was changed at this position
    Changed {
        index: usize,
        from: AssertionValue,
        to: AssertionValue,
    },
}

/// Result of diffing two collections
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectionDiff {
    pub items: Vec<CollectionDiffItem>,
    pub left_len: usize,
    pub right_len: usize,
}

impl CollectionDiff {
    /// Compute a diff between two lists
    pub fn compute_list(left: &[AssertionValue], right: &[AssertionValue]) -> Self {
        let mut items = Vec::new();
        let left_len = left.len();
        let right_len = right.len();

        let max_len = left_len.max(right_len);

        for i in 0..max_len {
            match (left.get(i), right.get(i)) {
                (Some(l), Some(r)) if l == r => {
                    items.push(CollectionDiffItem::Same {
                        index: i,
                        value: l.clone(),
                    });
                }
                (Some(l), Some(r)) => {
                    items.push(CollectionDiffItem::Changed {
                        index: i,
                        from: l.clone(),
                        to: r.clone(),
                    });
                }
                (Some(l), None) => {
                    items.push(CollectionDiffItem::Removed {
                        index: i,
                        value: l.clone(),
                    });
                }
                (None, Some(r)) => {
                    items.push(CollectionDiffItem::Added {
                        index: i,
                        value: r.clone(),
                    });
                }
                (None, None) => unreachable!(),
            }
        }

        Self {
            items,
            left_len,
            right_len,
        }
    }

    /// Compute a diff between two sets (unordered comparison)
    pub fn compute_set(left: &[AssertionValue], right: &[AssertionValue]) -> Self {
        let mut items = Vec::new();
        let left_len = left.len();
        let right_len = right.len();

        // Find items only in left (removed)
        for (i, l) in left.iter().enumerate() {
            if !right.contains(l) {
                items.push(CollectionDiffItem::Removed {
                    index: i,
                    value: l.clone(),
                });
            }
        }

        // Find items only in right (added)
        for (i, r) in right.iter().enumerate() {
            if !left.contains(r) {
                items.push(CollectionDiffItem::Added {
                    index: i,
                    value: r.clone(),
                });
            }
        }

        // Find items in both (same)
        for (i, l) in left.iter().enumerate() {
            if right.contains(l) {
                items.push(CollectionDiffItem::Same {
                    index: i,
                    value: l.clone(),
                });
            }
        }

        Self {
            items,
            left_len,
            right_len,
        }
    }

    /// Get only the differences (added, removed, changed)
    pub fn differences(&self) -> Vec<&CollectionDiffItem> {
        self.items
            .iter()
            .filter(|item| !matches!(item, CollectionDiffItem::Same { .. }))
            .collect()
    }

    /// Check if there are any differences
    pub fn has_differences(&self) -> bool {
        self.items.iter().any(|item| !matches!(item, CollectionDiffItem::Same { .. }))
    }

    /// Format the diff for display
    pub fn format(&self) -> String {
        let mut result = String::new();

        let added: Vec<_> = self
            .items
            .iter()
            .filter_map(|item| match item {
                CollectionDiffItem::Added { value, .. } => Some(value),
                _ => None,
            })
            .collect();

        let removed: Vec<_> = self
            .items
            .iter()
            .filter_map(|item| match item {
                CollectionDiffItem::Removed { value, .. } => Some(value),
                _ => None,
            })
            .collect();

        let changed: Vec<_> = self
            .items
            .iter()
            .filter_map(|item| match item {
                CollectionDiffItem::Changed { index, from, to } => Some((*index, from, to)),
                _ => None,
            })
            .collect();

        if !added.is_empty() {
            result.push_str("Added:\n");
            for value in added {
                result.push_str(&format!("  + {}\n", value));
            }
        }

        if !removed.is_empty() {
            result.push_str("Removed:\n");
            for value in removed {
                result.push_str(&format!("  - {}\n", value));
            }
        }

        if !changed.is_empty() {
            result.push_str("Changed:\n");
            for (index, from, to) in changed {
                result.push_str(&format!("  [{}]: {} -> {}\n", index, from, to));
            }
        }

        if result.is_empty() {
            result.push_str("No differences");
        }

        result
    }
}

/// Represents a difference in a dictionary entry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DictDiffItem {
    /// Key exists in both with same value
    Same { key: String, value: AssertionValue },
    /// Key was added (only in right)
    Added { key: String, value: AssertionValue },
    /// Key was removed (only in left)
    Removed { key: String, value: AssertionValue },
    /// Key exists in both but value changed
    Changed {
        key: String,
        from: AssertionValue,
        to: AssertionValue,
    },
}

/// Result of diffing two dictionaries
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DictDiff {
    pub items: Vec<DictDiffItem>,
}

impl DictDiff {
    /// Compute a diff between two dictionaries
    pub fn compute(left: &[(String, AssertionValue)], right: &[(String, AssertionValue)]) -> Self {
        let mut items = Vec::new();

        let left_map: std::collections::HashMap<_, _> = left.iter().cloned().collect();
        let right_map: std::collections::HashMap<_, _> = right.iter().cloned().collect();

        // Check all keys in left
        for (key, left_val) in &left_map {
            match right_map.get(key) {
                Some(right_val) if left_val == right_val => {
                    items.push(DictDiffItem::Same {
                        key: key.clone(),
                        value: left_val.clone(),
                    });
                }
                Some(right_val) => {
                    items.push(DictDiffItem::Changed {
                        key: key.clone(),
                        from: left_val.clone(),
                        to: right_val.clone(),
                    });
                }
                None => {
                    items.push(DictDiffItem::Removed {
                        key: key.clone(),
                        value: left_val.clone(),
                    });
                }
            }
        }

        // Check for keys only in right
        for (key, right_val) in &right_map {
            if !left_map.contains_key(key) {
                items.push(DictDiffItem::Added {
                    key: key.clone(),
                    value: right_val.clone(),
                });
            }
        }

        Self { items }
    }

    /// Get only the differences (added, removed, changed)
    pub fn differences(&self) -> Vec<&DictDiffItem> {
        self.items
            .iter()
            .filter(|item| !matches!(item, DictDiffItem::Same { .. }))
            .collect()
    }

    /// Check if there are any differences
    pub fn has_differences(&self) -> bool {
        self.items.iter().any(|item| !matches!(item, DictDiffItem::Same { .. }))
    }

    /// Format the diff for display
    pub fn format(&self) -> String {
        let mut result = String::new();

        let added: Vec<_> = self
            .items
            .iter()
            .filter_map(|item| match item {
                DictDiffItem::Added { key, value } => Some((key, value)),
                _ => None,
            })
            .collect();

        let removed: Vec<_> = self
            .items
            .iter()
            .filter_map(|item| match item {
                DictDiffItem::Removed { key, value } => Some((key, value)),
                _ => None,
            })
            .collect();

        let changed: Vec<_> = self
            .items
            .iter()
            .filter_map(|item| match item {
                DictDiffItem::Changed { key, from, to } => Some((key, from, to)),
                _ => None,
            })
            .collect();

        if !added.is_empty() {
            result.push_str("Added keys:\n");
            for (key, value) in added {
                result.push_str(&format!("  + '{}': {}\n", key, value));
            }
        }

        if !removed.is_empty() {
            result.push_str("Removed keys:\n");
            for (key, value) in removed {
                result.push_str(&format!("  - '{}': {}\n", key, value));
            }
        }

        if !changed.is_empty() {
            result.push_str("Changed values:\n");
            for (key, from, to) in changed {
                result.push_str(&format!("  '{}': {} -> {}\n", key, from, to));
            }
        }

        if result.is_empty() {
            result.push_str("No differences");
        }

        result
    }
}

/// Formatter for assertion failure messages with introspection
pub struct AssertionFormatter {
    use_colors: bool,
}

impl Default for AssertionFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl AssertionFormatter {
    /// Create a new formatter
    pub fn new() -> Self {
        Self { use_colors: true }
    }

    /// Create a formatter without colors
    pub fn plain() -> Self {
        Self { use_colors: false }
    }

    /// Format an assertion failure with full introspection
    pub fn format(&self, failure: &AssertionFailure) -> String {
        let mut result = String::new();

        // Header with location
        result.push_str(&format!("AssertionError at {}:{}\n", failure.file, failure.line));

        // Original expression
        result.push_str(&format!("  assert {}\n\n", failure.expression));

        // Custom message if present
        if let Some(ref msg) = failure.message {
            result.push_str(&format!("Message: {}\n\n", msg));
        }

        // Value introspection
        match (&failure.left, &failure.right, &failure.op) {
            (left, Some(right), Some(op)) => {
                result.push_str("Where:\n");
                result.push_str(&format!("  left  = {}\n", left));
                result.push_str(&format!("  right = {}\n", right));
                result.push_str(&format!("  op    = {}\n\n", op));

                // Add diff for comparable types
                if let Some(diff_str) = self.compute_diff(left, right) {
                    result.push_str("Diff:\n");
                    result.push_str(&diff_str);
                }
            }
            (left, None, None) => {
                result.push_str("Where:\n");
                result.push_str(&format!("  value = {} (falsy)\n", left));
            }
            _ => {}
        }

        result
    }

    /// Compute and format a diff between two values
    fn compute_diff(&self, left: &AssertionValue, right: &AssertionValue) -> Option<String> {
        match (left, right) {
            (AssertionValue::String(l), AssertionValue::String(r)) => {
                let diff = StringDiff::compute(l, r);
                if self.use_colors {
                    Some(format!("  {}\n", diff.format_colored()))
                } else {
                    Some(format!("  {}\n", diff.format_plain()))
                }
            }
            (AssertionValue::List(l), AssertionValue::List(r)) => {
                let diff = CollectionDiff::compute_list(l, r);
                if diff.has_differences() {
                    Some(diff.format())
                } else {
                    None
                }
            }
            (AssertionValue::Set(l), AssertionValue::Set(r)) => {
                let diff = CollectionDiff::compute_set(l, r);
                if diff.has_differences() {
                    Some(diff.format())
                } else {
                    None
                }
            }
            (AssertionValue::Dict(l), AssertionValue::Dict(r)) => {
                let diff = DictDiff::compute(l, r);
                if diff.has_differences() {
                    Some(diff.format())
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

impl fmt::Display for AssertionFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let formatter = AssertionFormatter::plain();
        write!(f, "{}", formatter.format(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_diff_equal() {
        let diff = StringDiff::compute("hello", "hello");
        assert_eq!(diff.segments.len(), 1);
        assert!(matches!(&diff.segments[0], DiffSegment::Equal(s) if s == "hello"));
    }

    #[test]
    fn test_string_diff_insertion() {
        let diff = StringDiff::compute("", "hello");
        assert_eq!(diff.segments.len(), 1);
        assert!(matches!(&diff.segments[0], DiffSegment::Inserted(s) if s == "hello"));
    }

    #[test]
    fn test_string_diff_deletion() {
        let diff = StringDiff::compute("hello", "");
        assert_eq!(diff.segments.len(), 1);
        assert!(matches!(&diff.segments[0], DiffSegment::Deleted(s) if s == "hello"));
    }

    #[test]
    fn test_string_diff_change() {
        let diff = StringDiff::compute("hello", "hallo");
        // Should have equal 'h', changed 'e'->'a', equal 'llo'
        assert!(diff.segments.len() >= 2);
    }

    #[test]
    fn test_collection_diff_same() {
        let left = vec![AssertionValue::Integer(1), AssertionValue::Integer(2)];
        let right = vec![AssertionValue::Integer(1), AssertionValue::Integer(2)];
        let diff = CollectionDiff::compute_list(&left, &right);
        assert!(!diff.has_differences());
    }

    #[test]
    fn test_collection_diff_added() {
        let left = vec![AssertionValue::Integer(1)];
        let right = vec![AssertionValue::Integer(1), AssertionValue::Integer(2)];
        let diff = CollectionDiff::compute_list(&left, &right);
        assert!(diff.has_differences());
        assert_eq!(diff.left_len, 1);
        assert_eq!(diff.right_len, 2);
    }

    #[test]
    fn test_dict_diff_same() {
        let left = vec![("a".to_string(), AssertionValue::Integer(1))];
        let right = vec![("a".to_string(), AssertionValue::Integer(1))];
        let diff = DictDiff::compute(&left, &right);
        assert!(!diff.has_differences());
    }

    #[test]
    fn test_dict_diff_changed() {
        let left = vec![("a".to_string(), AssertionValue::Integer(1))];
        let right = vec![("a".to_string(), AssertionValue::Integer(2))];
        let diff = DictDiff::compute(&left, &right);
        assert!(diff.has_differences());
    }

    #[test]
    fn test_assertion_failure_display() {
        let failure = AssertionFailure::comparison(
            "x == y",
            AssertionValue::Integer(1),
            ComparisonOp::Equal,
            AssertionValue::Integer(2),
            "test.py",
            10,
        );
        let output = failure.to_string();
        assert!(output.contains("AssertionError"));
        assert!(output.contains("test.py:10"));
        assert!(output.contains("x == y"));
    }

    #[test]
    fn test_assertion_value_display() {
        assert_eq!(AssertionValue::Integer(42).to_string(), "42");
        assert_eq!(AssertionValue::String("hello".to_string()).to_string(), "'hello'");
        assert_eq!(AssertionValue::Bool(true).to_string(), "True");
        assert_eq!(AssertionValue::None.to_string(), "None");
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate arbitrary assertion values
    fn arb_assertion_value() -> impl Strategy<Value = AssertionValue> {
        prop_oneof![
            any::<String>().prop_map(AssertionValue::String),
            any::<i64>().prop_map(AssertionValue::Integer),
            any::<f64>()
                .prop_filter("finite", |f| f.is_finite())
                .prop_map(AssertionValue::Float),
            any::<bool>().prop_map(AssertionValue::Bool),
            Just(AssertionValue::None),
        ]
    }

    /// Generate arbitrary comparison operators
    fn arb_comparison_op() -> impl Strategy<Value = ComparisonOp> {
        prop_oneof![
            Just(ComparisonOp::Equal),
            Just(ComparisonOp::NotEqual),
            Just(ComparisonOp::LessThan),
            Just(ComparisonOp::LessThanOrEqual),
            Just(ComparisonOp::GreaterThan),
            Just(ComparisonOp::GreaterThanOrEqual),
            Just(ComparisonOp::In),
            Just(ComparisonOp::NotIn),
            Just(ComparisonOp::Is),
            Just(ComparisonOp::IsNot),
        ]
    }

    /// Generate arbitrary assertion failures
    fn arb_assertion_failure() -> impl Strategy<Value = AssertionFailure> {
        (
            "[a-zA-Z0-9_ !=<>]+",                    // expression
            arb_assertion_value(),                   // left
            prop::option::of(arb_assertion_value()), // right
            prop::option::of(arb_comparison_op()),   // op
            prop::option::of("[a-zA-Z0-9 ]+"),       // message
            "[a-zA-Z0-9_/]+\\.py",                   // file
            1u32..10000u32,                          // line
        )
            .prop_map(|(expression, left, right, op, message, file, line)| {
                let mut failure = AssertionFailure {
                    expression,
                    left,
                    right,
                    op,
                    message: None,
                    file,
                    line,
                };
                if let Some(msg) = message {
                    failure = failure.with_message(msg);
                }
                failure
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-py-production-ready, Property 10: Assertion Introspection
        /// Validates: Requirements 13.1, 13.4
        ///
        /// Property: Assertion failure captures and preserves left and right values
        #[test]
        fn prop_assertion_failure_captures_values(
            expression in "[a-zA-Z0-9_ !=<>]+",
            left in arb_assertion_value(),
            right in arb_assertion_value(),
            op in arb_comparison_op(),
            file in "[a-zA-Z0-9_/]+\\.py",
            line in 1u32..10000u32,
        ) {
            let failure = AssertionFailure::comparison(
                expression.clone(),
                left.clone(),
                op,
                right.clone(),
                file.clone(),
                line,
            );

            // Verify left value is captured correctly
            prop_assert_eq!(failure.left_value(), &left);

            // Verify right value is captured correctly
            prop_assert_eq!(failure.right_value(), Some(&right));

            // Verify expression is preserved
            prop_assert_eq!(&failure.expression, &expression);

            // Verify file and line are preserved
            prop_assert_eq!(&failure.file, &file);
            prop_assert_eq!(failure.line, line);
        }

        /// Feature: dx-py-production-ready, Property 10: Assertion Introspection
        /// Validates: Requirements 13.1, 13.4
        ///
        /// Property: Assertion failure display includes expression and values
        #[test]
        fn prop_assertion_failure_display_includes_expression(
            failure in arb_assertion_failure(),
        ) {
            let display = failure.to_string();

            // Display should include the expression
            prop_assert!(
                display.contains(&failure.expression),
                "Display should contain expression '{}', got: {}",
                failure.expression,
                display
            );

            // Display should include file and line
            prop_assert!(
                display.contains(&failure.file),
                "Display should contain file '{}', got: {}",
                failure.file,
                display
            );
            prop_assert!(
                display.contains(&failure.line.to_string()),
                "Display should contain line '{}', got: {}",
                failure.line,
                display
            );
        }

        /// Feature: dx-py-production-ready, Property 10: Assertion Introspection
        /// Validates: Requirements 13.1, 13.4
        ///
        /// Property: Assertion failure serialization round-trip preserves all data
        #[test]
        fn prop_assertion_failure_roundtrip(failure in arb_assertion_failure()) {
            let serialized = bincode::serialize(&failure).expect("serialize");
            let deserialized: AssertionFailure = bincode::deserialize(&serialized).expect("deserialize");

            prop_assert_eq!(failure.expression, deserialized.expression);
            prop_assert_eq!(failure.left, deserialized.left);
            prop_assert_eq!(failure.right, deserialized.right);
            prop_assert_eq!(failure.op, deserialized.op);
            prop_assert_eq!(failure.message, deserialized.message);
            prop_assert_eq!(failure.file, deserialized.file);
            prop_assert_eq!(failure.line, deserialized.line);
        }

        /// Feature: dx-py-production-ready, Property 10: Assertion Introspection
        /// Validates: Requirements 13.2
        ///
        /// Property: String diff produces valid segments for any two strings
        #[test]
        fn prop_string_diff_valid_segments(
            left in "[a-zA-Z0-9 ]{0,50}",
            right in "[a-zA-Z0-9 ]{0,50}",
        ) {
            let diff = StringDiff::compute(&left, &right);

            // Diff should have at least one segment
            prop_assert!(!diff.segments.is_empty(), "Diff should have at least one segment");

            // If strings are equal, should have exactly one Equal segment
            if left == right {
                prop_assert_eq!(diff.segments.len(), 1);
                prop_assert!(matches!(&diff.segments[0], DiffSegment::Equal(s) if s == &left));
            }
        }

        /// Feature: dx-py-production-ready, Property 10: Assertion Introspection
        /// Validates: Requirements 13.3
        ///
        /// Property: Collection diff correctly identifies differences
        #[test]
        fn prop_collection_diff_identifies_differences(
            left in prop::collection::vec(any::<i64>().prop_map(AssertionValue::Integer), 0..10),
            right in prop::collection::vec(any::<i64>().prop_map(AssertionValue::Integer), 0..10),
        ) {
            let diff = CollectionDiff::compute_list(&left, &right);

            // If collections are equal, should have no differences
            if left == right {
                prop_assert!(!diff.has_differences(), "Equal collections should have no differences");
            }

            // Length should be recorded correctly
            prop_assert_eq!(diff.left_len, left.len());
            prop_assert_eq!(diff.right_len, right.len());
        }

        /// Feature: dx-py-production-ready, Property 10: Assertion Introspection
        /// Validates: Requirements 13.3
        ///
        /// Property: Dict diff correctly identifies key differences
        #[test]
        fn prop_dict_diff_identifies_key_differences(
            left in prop::collection::vec(
                ("[a-z]{1,5}", any::<i64>().prop_map(AssertionValue::Integer)),
                0..5
            ),
            right in prop::collection::vec(
                ("[a-z]{1,5}", any::<i64>().prop_map(AssertionValue::Integer)),
                0..5
            ),
        ) {
            let diff = DictDiff::compute(&left, &right);

            // If dicts are equal (same key-value pairs), should have no differences
            let left_map: std::collections::HashMap<_, _> = left.iter().cloned().collect();
            let right_map: std::collections::HashMap<_, _> = right.iter().cloned().collect();

            if left_map == right_map {
                prop_assert!(!diff.has_differences(), "Equal dicts should have no differences");
            }
        }
    }
}
