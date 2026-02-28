//! Pipeline Execution & Orchestration APIs

use anyhow::{Context, Result};
use parking_lot::RwLock;
use std::sync::{Arc, OnceLock};

#[cfg(test)]
use std::sync::Mutex;

/// Pipeline execution state
#[cfg(not(test))]
static PIPELINE_STATE: OnceLock<Arc<RwLock<PipelineState>>> = OnceLock::new();

// In test builds we keep pipeline state thread-local so that different tests
// (which may run on different worker threads) don't interfere with each
// other's view of pipeline suspension / active pipeline.
#[cfg(test)]
thread_local! {
    static TEST_PIPELINE_STATE: Arc<RwLock<PipelineState>> = Arc::new(RwLock::new(PipelineState::default()));
}

// When running tests, multiple test functions may touch the global
// pipeline APIs concurrently. To avoid flaky behaviour due to
// cross-test interference, we serialize pipeline operations behind a
// simple process-wide mutex. In production builds this guard is
// completely omitted.
#[cfg(test)]
static PIPELINE_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[cfg(test)]
fn pipeline_test_guard() -> std::sync::MutexGuard<'static, ()> {
    PIPELINE_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("pipeline test lock poisoned")
}

#[derive(Default)]
struct PipelineState {
    active_pipeline: Option<String>,
    execution_order: Vec<String>,
    is_suspended: bool,
    override_order: Option<Vec<String>>,
}

#[cfg(not(test))]
fn get_pipeline_state() -> Arc<RwLock<PipelineState>> {
    PIPELINE_STATE
        .get_or_init(|| Arc::new(RwLock::new(PipelineState::default())))
        .clone()
}

#[cfg(test)]
fn get_pipeline_state() -> Arc<RwLock<PipelineState>> {
    TEST_PIPELINE_STATE.with(|state| state.clone())
}

/// Executes named pipeline ("default" | "auth" | "deploy" | "ci")
pub fn execute_pipeline(pipeline_name: &str) -> Result<()> {
    #[cfg(test)]
    let _guard = pipeline_test_guard();

    let state = get_pipeline_state();
    let mut state = state.write();

    if state.is_suspended {
        anyhow::bail!("Pipeline execution is suspended");
    }

    tracing::info!("🎼 Executing pipeline: {}", pipeline_name);
    state.active_pipeline = Some(pipeline_name.to_string());

    // Get global forge instance
    let forge = crate::api::lifecycle::FORGE_INSTANCE
        .get()
        .ok_or_else(|| anyhow::anyhow!("Forge not initialized. Call initialize_forge() first."))?;

    // Execute tools via orchestrator
    let forge_guard = forge
        .lock()
        .map_err(|e| anyhow::anyhow!("Failed to lock forge instance: {}", e))?;
    let orchestrator = forge_guard.orchestrator();
    let mut orchestrator = orchestrator.write();

    // TODO: Filter tools based on pipeline name
    // For now, we execute all tools for the "default" pipeline
    if pipeline_name == "default" {
        let results = orchestrator
            .execute_all()
            .with_context(|| format!("Failed to execute pipeline: {}", pipeline_name))?;

        // Update state with execution order
        state.execution_order = results.iter()
            .map(|_o| "tool-id-placeholder".to_string()) // TODO: Get tool ID from output
            .collect();

        // Check for failures
        let failures: Vec<_> = results.iter().filter(|o| !o.success).collect();
        if !failures.is_empty() {
            anyhow::bail!("Pipeline execution failed: {} tools failed", failures.len());
        }
    } else {
        tracing::warn!("Pipeline '{}' not yet implemented, running default", pipeline_name);
        let _ = orchestrator.execute_all().with_context(|| {
            format!("Failed to execute default pipeline for: {}", pipeline_name)
        })?;
    }

    Ok(())
}

/// Highest priority execution — bypasses queue and debounce
pub fn execute_tool_immediately(tool_id: &str) -> Result<()> {
    #[cfg(test)]
    let _guard = pipeline_test_guard();

    tracing::info!("⚡ Immediate execution: {}", tool_id);

    // TODO: Execute tool directly, bypassing normal queue

    Ok(())
}

/// Returns final `Vec<ToolId>` after topology sort
pub fn get_resolved_execution_order() -> Result<Vec<String>> {
    #[cfg(test)]
    let _guard = pipeline_test_guard();

    let state = get_pipeline_state();
    let state = state.read();

    if let Some(override_order) = &state.override_order {
        Ok(override_order.clone())
    } else {
        Ok(state.execution_order.clone())
    }
}

/// Used by traffic_branching and user experiments
pub fn temporarily_override_pipeline_order(new_order: Vec<String>) -> Result<()> {
    #[cfg(test)]
    let _guard = pipeline_test_guard();

    let state = get_pipeline_state();
    let mut state = state.write();

    tracing::info!("🔀 Temporarily overriding pipeline order");
    state.override_order = Some(new_order);

    Ok(())
}

/// Aborts and restarts active pipeline from scratch
pub fn restart_current_pipeline() -> Result<()> {
    #[cfg(test)]
    let _guard = pipeline_test_guard();

    let state = get_pipeline_state();
    let state = state.read();

    if let Some(pipeline) = &state.active_pipeline {
        let name = pipeline.clone();
        drop(state);

        tracing::info!("🔄 Restarting pipeline: {}", name);
        execute_pipeline(&name).with_context(|| format!("Failed to restart pipeline: {}", name))?;
    } else {
        anyhow::bail!("No active pipeline to restart");
    }

    Ok(())
}

/// Pauses all tool execution until resumed
pub fn suspend_pipeline_execution() -> Result<()> {
    #[cfg(test)]
    let _guard = pipeline_test_guard();

    let state = get_pipeline_state();
    let mut state = state.write();

    tracing::info!("⏸️  Pipeline execution suspended");
    state.is_suspended = true;

    Ok(())
}

/// Continues from suspended state
pub fn resume_pipeline_execution() -> Result<()> {
    #[cfg(test)]
    let _guard = pipeline_test_guard();

    let state = get_pipeline_state();
    let mut state = state.write();

    tracing::info!("▶️  Pipeline execution resumed");
    state.is_suspended = false;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_execution() {
        // Pipeline execution requires Forge to be initialized
        // Without initialization, it should fail with "Forge not initialized"
        let result = execute_pipeline("default");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Forge not initialized"));
    }

    #[test]
    fn test_suspend_resume() {
        // Ensure we start with a clean state (not suspended)
        resume_pipeline_execution().unwrap();

        suspend_pipeline_execution().unwrap();
        // When suspended, execution should fail with "suspended" error
        let result = execute_pipeline("test");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("suspended"), "Expected 'suspended' error, got: {}", err_msg);

        resume_pipeline_execution().unwrap();
        // After resume, execution should either succeed (if Forge is initialized by another test)
        // or fail with "Forge not initialized" - but NOT with "suspended"
        let result = execute_pipeline("test");
        if let Err(e) = &result {
            let err_msg = e.to_string();
            // The key assertion: after resume, we should NOT get "suspended" error
            assert!(
                !err_msg.contains("suspended"),
                "Pipeline should not be suspended after resume, got: {}",
                err_msg
            );
        }
        // If result is Ok, that's also fine - it means Forge was initialized by another test
    }
}
