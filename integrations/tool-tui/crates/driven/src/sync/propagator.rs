//! Change propagation engine

use super::SyncEngine;
use crate::Result;
use std::path::Path;

/// Propagates changes to all enabled editors
#[derive(Debug)]
pub struct ChangePropagator<'a> {
    sync_engine: &'a SyncEngine,
}

impl<'a> ChangePropagator<'a> {
    /// Create a new propagator
    pub fn new(sync_engine: &'a SyncEngine) -> Self {
        Self { sync_engine }
    }

    /// Propagate changes from source to all targets
    pub fn propagate(&self, project_root: &Path) -> Result<PropagationResult> {
        let report = self.sync_engine.sync(project_root)?;

        Ok(PropagationResult {
            files_updated: report.synced_count(),
            errors: report.errors.len(),
        })
    }
}

/// Result of change propagation
#[derive(Debug)]
pub struct PropagationResult {
    /// Number of files updated
    pub files_updated: usize,
    /// Number of errors
    pub errors: usize,
}

impl PropagationResult {
    /// Check if propagation was successful
    pub fn is_success(&self) -> bool {
        self.errors == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_propagation_result() {
        let result = PropagationResult {
            files_updated: 3,
            errors: 0,
        };
        assert!(result.is_success());

        let failed = PropagationResult {
            files_updated: 1,
            errors: 2,
        };
        assert!(!failed.is_success());
    }
}
