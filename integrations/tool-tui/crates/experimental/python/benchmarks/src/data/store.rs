//! Result storage implementation

use crate::core::{BenchmarkConfig, BenchmarkResult, SystemInfo};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during result storage operations
#[derive(Debug, Error)]
pub enum StoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Result not found: {0}")]
    NotFound(String),

    #[error("Invalid result ID: {0}")]
    InvalidId(String),
}

/// Aggregated benchmark results for a suite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResults {
    pub suite: String,
    pub benchmarks: Vec<BenchmarkResult>,
    pub system_info: SystemInfo,
    pub config: BenchmarkConfig,
    pub timestamp: DateTime<Utc>,
}

impl BenchmarkResults {
    pub fn new(suite: impl Into<String>, config: BenchmarkConfig) -> Self {
        Self {
            suite: suite.into(),
            benchmarks: vec![],
            system_info: SystemInfo::collect(),
            config,
            timestamp: Utc::now(),
        }
    }

    pub fn add_result(&mut self, result: BenchmarkResult) {
        self.benchmarks.push(result);
    }
}

/// A stored benchmark result with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredResult {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub config: BenchmarkConfig,
    pub system_info: SystemInfo,
    pub results: BenchmarkResults,
}

impl StoredResult {
    /// Create a new stored result with a generated ID
    pub fn new(results: BenchmarkResults, config: BenchmarkConfig) -> Self {
        let timestamp = Utc::now();
        let id =
            format!("{}_{}_{}", results.suite, timestamp.format("%Y%m%d_%H%M%S"), &uuid_simple());

        Self {
            id,
            timestamp,
            config,
            system_info: results.system_info.clone(),
            results,
        }
    }

    /// Check if all required metadata is present
    pub fn has_complete_metadata(&self) -> bool {
        // Check config parameters are recorded
        self.config.measurement_iterations > 0
            && self.config.timeout_seconds > 0
            // Check timestamp is valid (not epoch)
            && self.timestamp.timestamp() > 0
    }
}

/// Generate a simple UUID-like string for result IDs
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    format!("{:x}{:x}", duration.as_secs(), duration.subsec_nanos())
}

/// Storage for benchmark results
pub struct ResultStore {
    pub storage_path: PathBuf,
}

impl ResultStore {
    pub fn new(storage_path: PathBuf) -> Self {
        Self { storage_path }
    }

    /// Ensure the storage directory exists
    fn ensure_dir(&self) -> Result<(), StoreError> {
        if !self.storage_path.exists() {
            fs::create_dir_all(&self.storage_path)?;
        }
        Ok(())
    }

    /// Get the file path for a result ID
    fn result_path(&self, id: &str) -> PathBuf {
        self.storage_path.join(format!("{}.json", id))
    }

    /// Save benchmark results to storage
    pub fn save(
        &self,
        results: &BenchmarkResults,
        config: &BenchmarkConfig,
    ) -> Result<String, StoreError> {
        self.ensure_dir()?;

        let stored = StoredResult::new(results.clone(), config.clone());
        let path = self.result_path(&stored.id);

        let json = serde_json::to_string_pretty(&stored)?;
        fs::write(&path, json)?;

        Ok(stored.id)
    }

    /// Load a stored result by ID
    pub fn load(&self, id: &str) -> Result<StoredResult, StoreError> {
        let path = self.result_path(id);

        if !path.exists() {
            return Err(StoreError::NotFound(id.to_string()));
        }

        let json = fs::read_to_string(&path)?;
        let stored: StoredResult = serde_json::from_str(&json)?;

        Ok(stored)
    }

    /// List recent stored results
    pub fn list_recent(&self, count: usize) -> Vec<StoredResult> {
        self.list_all().into_iter().take(count).collect()
    }

    /// List all stored results, sorted by timestamp (newest first)
    fn list_all(&self) -> Vec<StoredResult> {
        if !self.storage_path.exists() {
            return vec![];
        }

        let mut results: Vec<StoredResult> = fs::read_dir(&self.storage_path)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| entry.path().extension().map(|e| e == "json").unwrap_or(false))
                    .filter_map(|entry| {
                        let json = fs::read_to_string(entry.path()).ok()?;
                        serde_json::from_str(&json).ok()
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Sort by timestamp, newest first
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        results
    }

    /// Get historical results for a specific suite
    pub fn get_historical(&self, suite: &str, count: usize) -> Vec<StoredResult> {
        self.list_all()
            .into_iter()
            .filter(|r| r.results.suite == suite)
            .take(count)
            .collect()
    }

    /// Delete a stored result by ID
    pub fn delete(&self, id: &str) -> Result<(), StoreError> {
        let path = self.result_path(id);

        if !path.exists() {
            return Err(StoreError::NotFound(id.to_string()));
        }

        fs::remove_file(&path)?;
        Ok(())
    }

    /// Check if a result exists
    pub fn exists(&self, id: &str) -> bool {
        self.result_path(id).exists()
    }
}
