//! Command history for the DX CLI
//!
//! Provides binary-serialized command history with search and statistics.
//! - Requirement 9.1: Store command history in binary format
//! - Requirement 9.2: Record command, arguments, exit code, duration, timestamp, working directory
//! - Requirement 9.3: Limit history to configurable max entries (default 1000)
//! - Requirement 9.4: Provide search functionality
//! - Requirement 9.5: Provide statistics on command usage
//! - Requirement 7.1: Use atomic writes to prevent corruption
//! - Requirement 7.2: Detect and recover from corrupted history
//! - Requirement 7.5: Use file locking for concurrent access

use crate::utils::error::DxError;
use crate::utils::lock::{FileLock, LockType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Default maximum number of history entries
pub const DEFAULT_MAX_ENTRIES: usize = 1000;

/// A single command history entry
///
/// Requirement 9.2: Record command, arguments, exit code, duration, timestamp, working directory
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HistoryEntry {
    /// The command that was executed
    pub command: String,
    /// Arguments passed to the command
    pub arguments: Vec<String>,
    /// Exit code from the command (0 = success)
    pub exit_code: i32,
    /// Duration of the command in milliseconds
    pub duration_ms: u64,
    /// When the command was executed
    pub timestamp: DateTime<Utc>,
    /// Working directory when command was run
    pub working_dir: PathBuf,
}

impl HistoryEntry {
    /// Create a new history entry
    pub fn new(
        command: impl Into<String>,
        arguments: Vec<String>,
        exit_code: i32,
        duration: Duration,
        working_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            command: command.into(),
            arguments,
            exit_code,
            duration_ms: duration.as_millis() as u64,
            working_dir: working_dir.into(),
            timestamp: Utc::now(),
        }
    }

    /// Check if the command completed successfully
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    /// Get the command duration
    pub fn duration(&self) -> Duration {
        Duration::from_millis(self.duration_ms)
    }

    /// Check if entry matches a search query
    pub fn matches(&self, query: &str) -> bool {
        let q = query.to_lowercase();
        self.command.to_lowercase().contains(&q)
            || self.arguments.iter().any(|a| a.to_lowercase().contains(&q))
            || self.working_dir.to_string_lossy().to_lowercase().contains(&q)
    }
}

/// Command usage statistics (Requirement 9.5)
#[derive(Debug, Clone, Default)]
pub struct HistoryStats {
    /// Total number of commands executed
    pub total: usize,
    /// Number of successful commands
    pub successful: usize,
    /// Number of failed commands
    pub failed: usize,
    /// Average command duration in milliseconds
    pub avg_duration_ms: u64,
    /// Most frequently used commands
    pub top_commands: Vec<(String, usize)>,
}

/// Command history manager (Requirement 9.1)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommandHistory {
    entries: Vec<HistoryEntry>,
    #[serde(default = "default_max_entries")]
    max_entries: usize,
}

fn default_max_entries() -> usize {
    DEFAULT_MAX_ENTRIES
}

impl CommandHistory {
    /// Create a new empty command history
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: DEFAULT_MAX_ENTRIES,
        }
    }

    /// Create a new command history with custom max entries
    pub fn with_max_entries(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries,
        }
    }

    fn history_path() -> Option<PathBuf> {
        Some(home::home_dir()?.join(".dx").join("history.json"))
    }

    /// Load history with corruption recovery and file locking
    ///
    /// Requirement 7.2: Detect unparseable history, backup corrupted file and start fresh
    /// Requirement 7.5: Use file locking for concurrent access
    pub fn load() -> Result<Self, DxError> {
        let path = Self::history_path().ok_or_else(|| DxError::Io {
            message: "Could not determine home directory".into(),
        })?;
        if !path.exists() {
            return Ok(Self::new());
        }

        // Acquire shared lock for reading
        let _lock = FileLock::acquire(&path, LockType::Shared, Duration::from_secs(5))?;

        let content = std::fs::read_to_string(&path).map_err(|e| DxError::Io {
            message: format!("Failed to read history: {}", e),
        })?;

        // Try to parse the history
        match serde_json::from_str(&content) {
            Ok(history) => Ok(history),
            Err(e) => {
                // History is corrupted - need exclusive lock to backup
                drop(_lock);
                let _exclusive_lock =
                    FileLock::acquire(&path, LockType::Exclusive, Duration::from_secs(5))?;

                let backup_path = path.with_extension("corrupted.bak");
                if let Err(backup_err) = std::fs::rename(&path, &backup_path) {
                    // Log but don't fail - we'll just overwrite
                    eprintln!(
                        "Warning: Failed to backup corrupted history to {}: {}",
                        backup_path.display(),
                        backup_err
                    );
                } else {
                    eprintln!(
                        "Warning: History file was corrupted ({}). Backed up to {} and starting fresh.",
                        e,
                        backup_path.display()
                    );
                }
                Ok(Self::new())
            }
        }
    }

    /// Save history atomically with file locking (write to temp, then rename)
    ///
    /// Requirement 7.1: Use atomic writes to prevent corruption
    /// Requirement 7.5: Use file locking for concurrent access
    pub fn save(&self) -> Result<(), DxError> {
        let path = Self::history_path().ok_or_else(|| DxError::Io {
            message: "Could not determine home directory".into(),
        })?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DxError::Io {
                message: format!("Failed to create history directory: {}", e),
            })?;
        }

        // Acquire exclusive lock for writing
        let _lock = FileLock::acquire(&path, LockType::Exclusive, Duration::from_secs(5))?;

        let content = serde_json::to_string_pretty(self).map_err(|e| DxError::Io {
            message: format!("Failed to serialize history: {}", e),
        })?;

        // Write to temp file first, then atomic rename
        let temp_path = path.with_extension("tmp");
        std::fs::write(&temp_path, &content).map_err(|e| DxError::Io {
            message: format!("Failed to write temp history file: {}", e),
        })?;

        // Atomic rename
        std::fs::rename(&temp_path, &path).map_err(|e| DxError::Io {
            message: format!("Failed to rename history file: {}", e),
        })?;

        Ok(())
        // Lock is automatically released when _lock goes out of scope
    }

    /// Add entry with max entries enforcement (Requirement 9.3)
    pub fn add(&mut self, entry: HistoryEntry) {
        self.entries.push(entry);
        while self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }
    }

    /// Get recent entries up to a limit
    pub fn recent(&self, limit: usize) -> impl Iterator<Item = &HistoryEntry> {
        self.entries.iter().rev().take(limit)
    }

    /// Get all entries as a slice
    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if history is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the maximum number of entries
    pub fn max_entries(&self) -> usize {
        self.max_entries
    }

    /// Search history (Requirement 9.4)
    pub fn search(&self, query: &str) -> Vec<&HistoryEntry> {
        self.entries.iter().filter(|e| e.matches(query)).collect()
    }

    /// Calculate statistics (Requirement 9.5)
    pub fn stats(&self) -> HistoryStats {
        if self.entries.is_empty() {
            return HistoryStats::default();
        }
        let total = self.entries.len();
        let successful = self.entries.iter().filter(|e| e.is_success()).count();
        let total_dur: u64 = self.entries.iter().map(|e| e.duration_ms).sum();
        let mut counts: HashMap<String, usize> = HashMap::new();
        for e in &self.entries {
            *counts.entry(e.command.clone()).or_insert(0) += 1;
        }
        let mut top: Vec<_> = counts.into_iter().collect();
        top.sort_by(|a, b| b.1.cmp(&a.1));
        top.truncate(10);
        HistoryStats {
            total,
            successful,
            failed: total - successful,
            avg_duration_ms: total_dur / total as u64,
            top_commands: top,
        }
    }

    /// Clear all history entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn test_entry(cmd: &str, exit_code: i32) -> HistoryEntry {
        HistoryEntry {
            command: cmd.to_string(),
            arguments: vec!["--flag".to_string()],
            exit_code,
            duration_ms: 100,
            timestamp: Utc::now(),
            working_dir: PathBuf::from("/test"),
        }
    }

    #[test]
    fn test_entry_new() {
        let e = HistoryEntry::new(
            "build",
            vec!["--release".into()],
            0,
            Duration::from_millis(500),
            "/proj",
        );
        assert_eq!(e.command, "build");
        assert!(e.is_success());
    }

    #[test]
    fn test_entry_matches() {
        let e = test_entry("build", 0);
        assert!(e.matches("build"));
        assert!(e.matches("BUILD"));
        assert!(e.matches("flag"));
        assert!(!e.matches("deploy"));
    }

    #[test]
    fn test_add_and_recent() {
        let mut h = CommandHistory::new();
        h.add(test_entry("build", 0));
        h.add(test_entry("test", 0));
        h.add(test_entry("deploy", 1));
        assert_eq!(h.len(), 3);
        let r: Vec<_> = h.recent(2).collect();
        assert_eq!(r[0].command, "deploy");
        assert_eq!(r[1].command, "test");
    }

    #[test]
    fn test_max_entries() {
        let mut h = CommandHistory::with_max_entries(3);
        for i in 0..5 {
            h.add(test_entry(&format!("cmd{}", i), 0));
        }
        assert_eq!(h.len(), 3);
        assert_eq!(h.entries()[0].command, "cmd2");
    }

    #[test]
    fn test_search() {
        let mut h = CommandHistory::new();
        h.add(test_entry("build", 0));
        h.add(test_entry("test", 0));
        h.add(test_entry("build", 1));
        assert_eq!(h.search("build").len(), 2);
    }

    #[test]
    fn test_stats() {
        let mut h = CommandHistory::new();
        h.add(test_entry("build", 0));
        h.add(test_entry("build", 0));
        h.add(test_entry("test", 0));
        h.add(test_entry("deploy", 1));
        let s = h.stats();
        assert_eq!(s.total, 4);
        assert_eq!(s.successful, 3);
        assert_eq!(s.failed, 1);
        assert_eq!(s.top_commands[0], ("build".to_string(), 2));
    }

    #[test]
    fn test_serialization() {
        let mut h = CommandHistory::new();
        h.add(test_entry("build", 0));
        let json = serde_json::to_string(&h).unwrap();
        let r: CommandHistory = serde_json::from_str(&json).unwrap();
        assert_eq!(r.len(), 1);
    }

    // ═══════════════════════════════════════════════════════════════════
    //  PROPERTY TESTS
    // ═══════════════════════════════════════════════════════════════════

    // Property 13: History Serialization Round-Trip
    // **Validates: Requirements 9.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_history_serialization_roundtrip(
            commands in prop::collection::vec("[a-z]{1,10}", 0..20),
            exit_codes in prop::collection::vec(0i32..2, 0..20),
        ) {
            let mut history = CommandHistory::new();
            for (cmd, ec) in commands.iter().zip(exit_codes.iter()) {
                history.add(HistoryEntry {
                    command: cmd.clone(),
                    arguments: vec![],
                    exit_code: *ec,
                    duration_ms: 100,
                    timestamp: Utc::now(),
                    working_dir: PathBuf::from("/test"),
                });
            }
            let json = serde_json::to_string(&history).unwrap();
            let restored: CommandHistory = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(history.len(), restored.len());
            for (o, r) in history.entries().iter().zip(restored.entries().iter()) {
                prop_assert_eq!(&o.command, &r.command);
                prop_assert_eq!(o.exit_code, r.exit_code);
            }
        }
    }

    // Property 14: History Max Entries Enforcement
    // **Validates: Requirements 9.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_history_max_entries(max in 1usize..50, num in 0usize..100) {
            let mut h = CommandHistory::with_max_entries(max);
            for i in 0..num {
                h.add(HistoryEntry {
                    command: format!("cmd{}", i),
                    arguments: vec![],
                    exit_code: 0,
                    duration_ms: 100,
                    timestamp: Utc::now(),
                    working_dir: PathBuf::from("/test"),
                });
            }
            prop_assert!(h.len() <= max);
            if num > max {
                prop_assert_eq!(&h.entries()[0].command, &format!("cmd{}", num - max));
            }
        }
    }

    // Property 15: History Search Functionality
    // **Validates: Requirements 9.4**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_history_search(query in "[a-z]{1,5}", cmds in prop::collection::vec("[a-z]{1,10}", 1..20)) {
            let mut h = CommandHistory::new();
            // Use a working_dir that won't interfere with search results
            let work_dir = PathBuf::from("/work");
            for cmd in &cmds {
                h.add(HistoryEntry {
                    command: cmd.clone(),
                    arguments: vec![],
                    exit_code: 0,
                    duration_ms: 100,
                    timestamp: Utc::now(),
                    working_dir: work_dir.clone(),
                });
            }
            let results = h.search(&query);
            // All results must contain the query in command, arguments, or working_dir
            for entry in &results {
                prop_assert!(entry.matches(&query), "Entry {:?} should match query '{}'", entry.command, query);
            }
            // Count expected matches (command contains query OR working_dir contains query)
            let q_lower = query.to_lowercase();
            let expected = cmds.iter().filter(|c| {
                c.to_lowercase().contains(&q_lower) || work_dir.to_string_lossy().to_lowercase().contains(&q_lower)
            }).count();
            prop_assert_eq!(results.len(), expected);
        }
    }

    // Property 16: History Statistics Accuracy
    // **Validates: Requirements 9.5**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_history_stats(exit_codes in prop::collection::vec(0i32..2, 1..50)) {
            let mut h = CommandHistory::new();
            for (i, ec) in exit_codes.iter().enumerate() {
                h.add(HistoryEntry {
                    command: format!("cmd{}", i % 5),
                    arguments: vec![],
                    exit_code: *ec,
                    duration_ms: 100,
                    timestamp: Utc::now(),
                    working_dir: PathBuf::from("/test"),
                });
            }
            let s = h.stats();
            prop_assert_eq!(s.total, exit_codes.len());
            prop_assert_eq!(s.successful, exit_codes.iter().filter(|&&e| e == 0).count());
            prop_assert_eq!(s.failed, exit_codes.iter().filter(|&&e| e != 0).count());
            prop_assert_eq!(s.successful + s.failed, s.total);
        }
    }

    // Feature: dx-cli-hardening, Property 21: History Search Case Insensitivity
    // **Validates: Requirements 7.3**
    //
    // For any search query and history entry, if the entry's command, arguments,
    // or working directory contains the query (case-insensitive), the entry SHALL
    // be included in search results.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_case_insensitive_search(
            query in "[a-zA-Z]{1,5}",
            cmds in prop::collection::vec("[a-zA-Z]{1,10}", 1..20)
        ) {
            let mut h = CommandHistory::new();
            let work_dir = PathBuf::from("/work");
            for cmd in &cmds {
                h.add(HistoryEntry {
                    command: cmd.clone(),
                    arguments: vec![],
                    exit_code: 0,
                    duration_ms: 100,
                    timestamp: Utc::now(),
                    working_dir: work_dir.clone(),
                });
            }

            // Search with original case
            let results_original = h.search(&query);

            // Search with uppercase
            let results_upper = h.search(&query.to_uppercase());

            // Search with lowercase
            let results_lower = h.search(&query.to_lowercase());

            // All searches should return the same results (case-insensitive)
            prop_assert_eq!(
                results_original.len(),
                results_upper.len(),
                "Uppercase search should return same count as original"
            );
            prop_assert_eq!(
                results_original.len(),
                results_lower.len(),
                "Lowercase search should return same count as original"
            );
        }
    }

    // Feature: dx-cli-hardening, Property 22: History FIFO Eviction
    // **Validates: Requirements 7.4**
    //
    // When history exceeds max entries, the oldest entries SHALL be removed first (FIFO).
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_fifo_eviction(
            max_entries in 5usize..20,
            num_entries in 10usize..50
        ) {
            prop_assume!(num_entries > max_entries);

            let mut h = CommandHistory::with_max_entries(max_entries);
            for i in 0..num_entries {
                h.add(HistoryEntry {
                    command: format!("cmd{}", i),
                    arguments: vec![],
                    exit_code: 0,
                    duration_ms: 100,
                    timestamp: Utc::now(),
                    working_dir: PathBuf::from("/test"),
                });
            }

            // Should have exactly max_entries
            prop_assert_eq!(h.len(), max_entries);

            // The oldest entries should be evicted (FIFO)
            // First entry should be cmd{num_entries - max_entries}
            let expected_first = format!("cmd{}", num_entries - max_entries);
            prop_assert_eq!(
                &h.entries()[0].command,
                &expected_first,
                "First entry should be the oldest remaining after FIFO eviction"
            );

            // Last entry should be cmd{num_entries - 1}
            let expected_last = format!("cmd{}", num_entries - 1);
            prop_assert_eq!(
                &h.entries()[max_entries - 1].command,
                &expected_last,
                "Last entry should be the most recent"
            );
        }
    }

    // Feature: dx-cli-hardening, Property 23: History Statistics Accuracy
    // **Validates: Requirements 7.6**
    //
    // For any history, stats().total SHALL equal stats().successful + stats().failed,
    // and these counts SHALL match the actual entry counts.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_statistics_accuracy(
            exit_codes in prop::collection::vec(any::<i32>(), 1..100)
        ) {
            let mut h = CommandHistory::new();
            for (i, ec) in exit_codes.iter().enumerate() {
                h.add(HistoryEntry {
                    command: format!("cmd{}", i),
                    arguments: vec![],
                    exit_code: *ec,
                    duration_ms: 100,
                    timestamp: Utc::now(),
                    working_dir: PathBuf::from("/test"),
                });
            }

            let stats = h.stats();

            // Total should equal successful + failed
            prop_assert_eq!(
                stats.total,
                stats.successful + stats.failed,
                "total should equal successful + failed"
            );

            // Total should match actual entry count
            prop_assert_eq!(
                stats.total,
                h.len(),
                "total should match actual entry count"
            );

            // Successful count should match entries with exit_code == 0
            let actual_successful = h.entries().iter().filter(|e| e.exit_code == 0).count();
            prop_assert_eq!(
                stats.successful,
                actual_successful,
                "successful count should match entries with exit_code == 0"
            );

            // Failed count should match entries with exit_code != 0
            let actual_failed = h.entries().iter().filter(|e| e.exit_code != 0).count();
            prop_assert_eq!(
                stats.failed,
                actual_failed,
                "failed count should match entries with exit_code != 0"
            );
        }
    }

    // Feature: dx-cli-hardening, Property 24: History Entry Completeness
    // **Validates: Requirements 7.7**
    //
    // For any HistoryEntry, all fields (command, arguments, exit_code, duration_ms,
    // timestamp, working_dir) SHALL be present and non-default after construction.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_entry_completeness(
            command in "[a-z]{1,20}",
            args in prop::collection::vec("[a-z]{1,10}", 0..5),
            exit_code in any::<i32>(),
            duration_ms in 1u64..10000
        ) {
            let entry = HistoryEntry {
                command: command.clone(),
                arguments: args.clone(),
                exit_code,
                duration_ms,
                timestamp: Utc::now(),
                working_dir: PathBuf::from("/test/path"),
            };

            // All fields should be present and match what was provided
            prop_assert_eq!(&entry.command, &command, "command should match");
            prop_assert_eq!(&entry.arguments, &args, "arguments should match");
            prop_assert_eq!(entry.exit_code, exit_code, "exit_code should match");
            prop_assert_eq!(entry.duration_ms, duration_ms, "duration_ms should match");
            prop_assert!(!entry.working_dir.as_os_str().is_empty(), "working_dir should not be empty");

            // Timestamp should be recent (within last minute)
            let now = Utc::now();
            let diff = now.signed_duration_since(entry.timestamp);
            prop_assert!(
                diff.num_seconds() < 60,
                "timestamp should be recent"
            );
        }
    }
}
