//! Memory Resume for Interactive Sessions - Feature #14
//!
//! Generation sessions can be paused and resumed with binary snapshots.
//! Zero re-computation when navigating wizard steps.

use crate::error::{GeneratorError, Result};
use crate::params::{ParamValue, Parameters};
use std::path::Path;

// ============================================================================
// Session Step
// ============================================================================

/// A step in an interactive generation session.
#[derive(Clone, Debug)]
pub struct SessionStep {
    /// Step index (0-based).
    pub index: usize,
    /// Step name/title.
    pub name: String,
    /// Step description.
    pub description: String,
    /// Parameter names collected at this step.
    pub params: Vec<String>,
    /// Whether this step is complete.
    pub complete: bool,
}

impl SessionStep {
    /// Create a new step.
    #[must_use]
    pub fn new(index: usize, name: impl Into<String>) -> Self {
        Self {
            index,
            name: name.into(),
            description: String::new(),
            params: Vec::new(),
            complete: false,
        }
    }

    /// Set description.
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add a parameter.
    #[must_use]
    pub fn with_param(mut self, param: impl Into<String>) -> Self {
        self.params.push(param.into());
        self
    }
}

// ============================================================================
// Session Snapshot
// ============================================================================

/// Binary snapshot of session state for persistence.
#[derive(Clone, Debug)]
pub struct SessionSnapshot {
    /// Session ID.
    pub session_id: String,
    /// Template name.
    pub template_name: String,
    /// Current step index.
    pub current_step: usize,
    /// Collected parameters (DX ∞ encoded).
    pub params_data: Vec<u8>,
    /// Step completion flags.
    pub step_flags: Vec<bool>,
    /// Timestamp.
    pub timestamp: u64,
}

impl SessionSnapshot {
    /// Serialize to bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();

        // Magic
        out.extend_from_slice(b"DXSS"); // DX Session Snapshot

        // Session ID
        let id_bytes = self.session_id.as_bytes();
        out.extend_from_slice(&(id_bytes.len() as u16).to_le_bytes());
        out.extend_from_slice(id_bytes);

        // Template name
        let name_bytes = self.template_name.as_bytes();
        out.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        out.extend_from_slice(name_bytes);

        // Current step
        out.extend_from_slice(&(self.current_step as u16).to_le_bytes());

        // Params data
        out.extend_from_slice(&(self.params_data.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.params_data);

        // Step flags
        out.push(self.step_flags.len() as u8);
        for flag in &self.step_flags {
            out.push(if *flag { 1 } else { 0 });
        }

        // Timestamp
        out.extend_from_slice(&self.timestamp.to_le_bytes());

        out
    }

    /// Deserialize from bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 4 || &data[0..4] != b"DXSS" {
            return Err(GeneratorError::SessionCorrupted {
                reason: "Invalid magic".to_string(),
            });
        }

        let mut offset = 4;

        // Session ID
        let id_len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;
        let session_id = String::from_utf8_lossy(&data[offset..offset + id_len]).into_owned();
        offset += id_len;

        // Template name
        let name_len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;
        let template_name = String::from_utf8_lossy(&data[offset..offset + name_len]).into_owned();
        offset += name_len;

        // Current step
        let current_step = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;

        // Params data
        let params_len = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]) as usize;
        offset += 4;
        let params_data = data[offset..offset + params_len].to_vec();
        offset += params_len;

        // Step flags
        let flags_count = data[offset] as usize;
        offset += 1;
        let mut step_flags = Vec::with_capacity(flags_count);
        for i in 0..flags_count {
            step_flags.push(data[offset + i] != 0);
        }
        offset += flags_count;

        // Timestamp
        let timestamp = u64::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]);

        Ok(Self {
            session_id,
            template_name,
            current_step,
            params_data,
            step_flags,
            timestamp,
        })
    }
}

// ============================================================================
// Session
// ============================================================================

/// Interactive generation session with resume support.
///
/// Tracks multi-step parameter collection with instant
/// forward/backward navigation.
#[derive(Clone, Debug)]
pub struct Session {
    /// Session ID.
    id: String,
    /// Template name.
    template_name: String,
    /// Session steps.
    steps: Vec<SessionStep>,
    /// Current step index.
    current_step: usize,
    /// Collected parameters.
    params: Parameters<'static>,
    /// Step navigation history (for back).
    history: Vec<usize>,
}

impl Session {
    /// Create a new session.
    #[must_use]
    pub fn new(template_name: impl Into<String>) -> Self {
        let id = format!(
            "{:016x}",
            xxhash_rust::xxh64::xxh64(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
                    .to_le_bytes()
                    .as_slice(),
                0,
            )
        );

        Self {
            id,
            template_name: template_name.into(),
            steps: Vec::new(),
            current_step: 0,
            params: Parameters::new(),
            history: Vec::new(),
        }
    }

    /// Get session ID.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get template name.
    #[must_use]
    pub fn template_name(&self) -> &str {
        &self.template_name
    }

    /// Add a step.
    pub fn add_step(&mut self, step: SessionStep) {
        self.steps.push(step);
    }

    /// Get current step.
    #[must_use]
    pub fn current(&self) -> Option<&SessionStep> {
        self.steps.get(self.current_step)
    }

    /// Get current step index.
    #[must_use]
    pub fn current_index(&self) -> usize {
        self.current_step
    }

    /// Get total step count.
    #[must_use]
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Check if session is complete.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.steps.iter().all(|s| s.complete)
    }

    /// Move to next step.
    pub fn next(&mut self) -> bool {
        if self.current_step + 1 < self.steps.len() {
            self.history.push(self.current_step);
            self.current_step += 1;
            true
        } else {
            false
        }
    }

    /// Move to previous step.
    pub fn back(&mut self) -> bool {
        if let Some(prev) = self.history.pop() {
            self.current_step = prev;
            true
        } else if self.current_step > 0 {
            self.current_step -= 1;
            true
        } else {
            false
        }
    }

    /// Go to a specific step.
    pub fn goto(&mut self, step: usize) -> bool {
        if step < self.steps.len() {
            self.history.push(self.current_step);
            self.current_step = step;
            true
        } else {
            false
        }
    }

    /// Set a parameter value.
    pub fn set_param(&mut self, name: impl Into<String>, value: impl Into<ParamValue<'static>>) {
        self.params.insert(name.into(), value.into());
    }

    /// Get collected parameters.
    #[must_use]
    pub fn params(&self) -> &Parameters<'static> {
        &self.params
    }

    /// Mark current step as complete.
    pub fn complete_current(&mut self) {
        if let Some(step) = self.steps.get_mut(self.current_step) {
            step.complete = true;
        }
    }

    /// Create a snapshot for persistence.
    #[must_use]
    pub fn snapshot(&self) -> SessionSnapshot {
        let step_flags = self.steps.iter().map(|s| s.complete).collect();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        SessionSnapshot {
            session_id: self.id.clone(),
            template_name: self.template_name.clone(),
            current_step: self.current_step,
            params_data: self.params.encode(),
            step_flags,
            timestamp,
        }
    }

    /// Save session to file.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let snapshot = self.snapshot();
        std::fs::write(path, snapshot.to_bytes())?;
        Ok(())
    }

    /// Load session from file.
    pub fn load(path: impl AsRef<Path>) -> Result<SessionSnapshot> {
        let data = std::fs::read(path)?;
        SessionSnapshot::from_bytes(&data)
    }

    /// Get progress as percentage.
    #[must_use]
    pub fn progress(&self) -> f64 {
        if self.steps.is_empty() {
            return 100.0;
        }
        let complete = self.steps.iter().filter(|s| s.complete).count();
        (complete as f64 / self.steps.len() as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_navigation() {
        let mut session = Session::new("component");

        session.add_step(SessionStep::new(0, "Name"));
        session.add_step(SessionStep::new(1, "Options"));
        session.add_step(SessionStep::new(2, "Confirm"));

        assert_eq!(session.current_index(), 0);

        assert!(session.next());
        assert_eq!(session.current_index(), 1);

        assert!(session.next());
        assert_eq!(session.current_index(), 2);

        assert!(!session.next()); // Can't go past end

        assert!(session.back());
        assert_eq!(session.current_index(), 1);
    }

    #[test]
    fn test_session_params() {
        let mut session = Session::new("test");

        session.set_param("name", "Counter");
        session.set_param("count", 42i32);

        let params = session.params();
        assert_eq!(params.get("name").unwrap().as_str(), Some("Counter"));
        assert_eq!(params.get("count").unwrap().as_int(), Some(42));
    }

    #[test]
    fn test_session_snapshot() {
        let mut session = Session::new("test");

        session.add_step(SessionStep::new(0, "Step 1"));
        session.add_step(SessionStep::new(1, "Step 2"));
        session.set_param("name", "Test");
        session.complete_current();
        session.next();

        let snapshot = session.snapshot();

        assert_eq!(snapshot.template_name, "test");
        assert_eq!(snapshot.current_step, 1);
        assert_eq!(snapshot.step_flags, vec![true, false]);

        // Round-trip
        let bytes = snapshot.to_bytes();
        let restored = SessionSnapshot::from_bytes(&bytes).unwrap();

        assert_eq!(restored.session_id, snapshot.session_id);
        assert_eq!(restored.current_step, 1);
    }

    #[test]
    fn test_session_progress() {
        let mut session = Session::new("test");

        session.add_step(SessionStep::new(0, "Step 1"));
        session.add_step(SessionStep::new(1, "Step 2"));

        assert_eq!(session.progress(), 0.0);

        session.complete_current();
        assert_eq!(session.progress(), 50.0);

        session.next();
        session.complete_current();
        assert_eq!(session.progress(), 100.0);
        assert!(session.is_complete());
    }
}
