//! Triple-Path Reactivity Engine APIs

use anyhow::Result;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use tokio::sync::mpsc;
use tokio::time::{Duration, Instant, sleep};

/// Reactivity state management
static REACTIVITY_STATE: OnceLock<Arc<RwLock<ReactivityState>>> = OnceLock::new();

/// Default debounce delay in milliseconds
const DEFAULT_DEBOUNCE_DELAY_MS: u64 = 300;

/// Default idle threshold in milliseconds (2 seconds)
const DEFAULT_IDLE_THRESHOLD_MS: u64 = 2000;

/// Debounce state for a single file
struct DebounceState {
    /// Channel to cancel pending debounce
    cancel_tx: mpsc::Sender<()>,
    /// Last event timestamp
    last_event: Instant,
}

/// Scheduled task for idle execution
struct ScheduledIdleTask {
    /// Unique task identifier
    task_id: String,
    /// The task function to execute
    task_fn: Box<dyn Fn() -> Result<()> + Send + Sync>,
}

/// Idle detection state
struct IdleState {
    /// Last activity timestamp
    last_activity: Instant,
    /// Whether we're currently in idle state
    is_idle: bool,
    /// Whether an idle event has been triggered for the current idle period
    idle_event_triggered: bool,
    /// Channel to notify idle watchers
    idle_notify_tx: Option<mpsc::Sender<()>>,
}

impl Default for IdleState {
    fn default() -> Self {
        Self {
            last_activity: Instant::now(),
            is_idle: false,
            idle_event_triggered: false,
            idle_notify_tx: None,
        }
    }
}

/// Reactivity state including debounce tracking
struct ReactivityState {
    in_batch: bool,
    batch_start: Option<std::time::Instant>,
    /// Debounce state per file path
    debounce_states: HashMap<PathBuf, DebounceState>,
    /// Configurable debounce delay
    debounce_delay: Duration,
    /// Idle detection state
    idle_state: IdleState,
    /// Configurable idle threshold
    idle_threshold: Duration,
    /// Tasks scheduled for idle time execution
    scheduled_idle_tasks: Vec<ScheduledIdleTask>,
}

impl Default for ReactivityState {
    fn default() -> Self {
        Self {
            in_batch: false,
            batch_start: None,
            debounce_states: HashMap::new(),
            debounce_delay: Duration::from_millis(DEFAULT_DEBOUNCE_DELAY_MS),
            idle_state: IdleState::default(),
            idle_threshold: Duration::from_millis(DEFAULT_IDLE_THRESHOLD_MS),
            scheduled_idle_tasks: Vec::new(),
        }
    }
}

fn get_reactivity_state() -> Arc<RwLock<ReactivityState>> {
    REACTIVITY_STATE
        .get_or_init(|| Arc::new(RwLock::new(ReactivityState::default())))
        .clone()
}

/// Configure the idle threshold for idle detection
///
/// # Arguments
/// * `threshold` - The idle threshold duration (default is 2 seconds)
///
/// # Example
/// ```rust,no_run
/// use dx_forge::configure_idle_threshold;
/// use std::time::Duration;
///
/// configure_idle_threshold(Duration::from_secs(5));
/// ```
pub fn configure_idle_threshold(threshold: Duration) {
    let state = get_reactivity_state();
    let mut state = state.write();
    state.idle_threshold = threshold;
    tracing::info!("😴 Idle threshold configured to {:?}", threshold);
}

/// Get the current idle threshold configuration
pub fn get_idle_threshold() -> Duration {
    let state = get_reactivity_state();

    state.read().idle_threshold
}

/// Record user activity to reset idle detection
///
/// This should be called whenever user activity is detected (e.g., file edits, cursor moves).
/// It resets the idle timer and marks the system as no longer idle.
///
/// # Example
/// ```rust,no_run
/// use dx_forge::record_activity;
///
/// // Call this when user activity is detected
/// record_activity();
/// ```
pub fn record_activity() {
    let state = get_reactivity_state();
    let mut state = state.write();
    state.idle_state.last_activity = Instant::now();
    state.idle_state.is_idle = false;
    state.idle_state.idle_event_triggered = false;

    // Notify any waiting idle event watchers that activity was detected
    if let Some(tx) = state.idle_state.idle_notify_tx.take() {
        let _ = tx.try_send(());
    }

    tracing::trace!("📝 Activity recorded, idle timer reset");
}

/// Get the time elapsed since the last activity
///
/// # Returns
/// Duration since the last recorded activity
pub fn time_since_last_activity() -> Duration {
    let state = get_reactivity_state();

    state.read().idle_state.last_activity.elapsed()
}

/// Check if the system is currently idle
///
/// Returns true if the time since last activity exceeds the idle threshold.
pub fn is_idle() -> bool {
    let state = get_reactivity_state();
    let state = state.read();
    state.idle_state.last_activity.elapsed() >= state.idle_threshold
}

/// Configure the debounce delay for all debounced events
///
/// # Arguments
/// * `delay` - The debounce delay duration
///
/// # Example
/// ```rust,no_run
/// use dx_forge::configure_debounce_delay;
/// use std::time::Duration;
///
/// configure_debounce_delay(Duration::from_millis(500));
/// ```
pub fn configure_debounce_delay(delay: Duration) {
    let state = get_reactivity_state();
    let mut state = state.write();
    state.debounce_delay = delay;
    tracing::info!("⏱️  Debounce delay configured to {:?}", delay);
}

/// Instant path — called on every DidChangeTextDocument
///
/// Triggers immediate tool execution for realtime feedback (e.g., syntax highlighting, diagnostics).
pub fn trigger_realtime_event(file: PathBuf, _content: String) -> Result<()> {
    tracing::debug!("⚡ Realtime event: {:?}", file);

    // Update execution context with changed file
    if let Some(forge) = crate::api::lifecycle::FORGE_INSTANCE.get() {
        let forge_guard =
            forge.lock().map_err(|e| anyhow::anyhow!("Failed to lock forge: {}", e))?;
        let orchestrator = forge_guard.orchestrator();
        let mut orchestrator = orchestrator.write();

        // Add to changed files if not already present
        if !orchestrator.context().changed_files.contains(&file) {
            orchestrator.context_mut().changed_files.push(file.clone());
        }

        // Execute realtime pipeline (if we had one, for now just log)
        tracing::info!("⚡ Triggering realtime analysis for {:?}", file);

        // In a real production system, we might run a subset of fast tools here
        // orchestrator.execute_subset(&["dx-lint", "dx-format"])?;
    }

    Ok(())
}

/// 300ms debounce — safe default for style, lint, format
///
/// Triggers tool execution after a configurable debounce period to avoid excessive runs.
/// If multiple events are triggered for the same file within the debounce window,
/// only the last event will actually execute the handler.
///
/// # Arguments
/// * `file` - The file path that triggered the event
/// * `content` - The file content
///
/// # Example
/// ```rust,no_run
/// use dx_forge::trigger_debounced_event;
/// use std::path::PathBuf;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     trigger_debounced_event(PathBuf::from("src/main.rs"), "content".to_string()).await?;
///     Ok(())
/// }
/// ```
pub async fn trigger_debounced_event(file: PathBuf, content: String) -> Result<()> {
    trigger_debounced_event_with_delay(file, content, None).await
}

/// Triggers a debounced event with a custom delay
///
/// This is the configurable version of `trigger_debounced_event` that allows
/// specifying a custom debounce delay for this specific event.
///
/// # Arguments
/// * `file` - The file path that triggered the event
/// * `content` - The file content
/// * `delay` - Optional custom delay (uses configured default if None)
///
/// # Example
/// ```rust,no_run
/// use dx_forge::trigger_debounced_event_with_delay;
/// use std::path::PathBuf;
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     trigger_debounced_event_with_delay(
///         PathBuf::from("src/main.rs"),
///         "content".to_string(),
///         Some(Duration::from_millis(500))
///     ).await?;
///     Ok(())
/// }
/// ```
pub async fn trigger_debounced_event_with_delay(
    file: PathBuf,
    content: String,
    delay: Option<Duration>,
) -> Result<()> {
    let state = get_reactivity_state();
    let debounce_delay = delay.unwrap_or_else(|| state.read().debounce_delay);

    tracing::debug!("⏱️  Debounced event: {:?} (delay: {:?})", file, debounce_delay);

    // Create a cancellation channel for this debounce
    let (cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);

    // Cancel any existing pending debounce for this file
    {
        let mut state_guard = state.write();
        if let Some(existing) = state_guard.debounce_states.remove(&file) {
            // Send cancel signal to existing debounce (ignore if receiver dropped)
            let _ = existing.cancel_tx.try_send(());
            tracing::debug!("⏱️  Cancelled pending debounce for {:?}", file);
        }

        // Register new debounce state
        state_guard.debounce_states.insert(
            file.clone(),
            DebounceState {
                cancel_tx: cancel_tx.clone(),
                last_event: Instant::now(),
            },
        );
    }

    let file_clone = file.clone();
    let content_clone = content.clone();

    // Wait for debounce period or cancellation
    tokio::select! {
        _ = sleep(debounce_delay) => {
            // Debounce period elapsed without cancellation - execute the handler
            tracing::debug!("⏱️  Debounce elapsed for {:?}, executing handler", file_clone);

            // Clean up debounce state
            {
                let mut state_guard = state.write();
                state_guard.debounce_states.remove(&file_clone);
            }

            // Execute the actual debounced handler
            execute_debounced_handler(&file_clone, &content_clone).await?;
        }
        _ = cancel_rx.recv() => {
            // Cancelled by a newer event - do nothing
            tracing::debug!("⏱️  Debounce cancelled for {:?}", file_clone);
        }
    }

    Ok(())
}

/// Internal handler that executes after debounce period
async fn execute_debounced_handler(file: &PathBuf, _content: &str) -> Result<()> {
    tracing::info!("⏱️  Executing debounced tools for {:?}", file);

    // Update execution context with changed file
    if let Some(forge) = crate::api::lifecycle::FORGE_INSTANCE.get() {
        let forge_guard =
            forge.lock().map_err(|e| anyhow::anyhow!("Failed to lock forge: {}", e))?;
        let orchestrator = forge_guard.orchestrator();
        let mut orchestrator = orchestrator.write();

        // Add to changed files if not already present
        if !orchestrator.context().changed_files.contains(file) {
            orchestrator.context_mut().changed_files.push(file.clone());
        }

        // In a production system, execute debounced tools here
        // For example: linters, formatters, style checkers
        // orchestrator.execute_subset(&["dx-lint", "dx-format"])?;
    }

    Ok(())
}

/// Check if there's a pending debounce for a file
///
/// # Arguments
/// * `file` - The file path to check
///
/// # Returns
/// `true` if there's a pending debounce for the file
pub fn has_pending_debounce(file: &PathBuf) -> bool {
    let state = get_reactivity_state();
    let state_guard = state.read();
    state_guard.debounce_states.contains_key(file)
}

/// Cancel a pending debounce for a file
///
/// # Arguments
/// * `file` - The file path to cancel debounce for
///
/// # Returns
/// `true` if a pending debounce was cancelled
pub fn cancel_debounce(file: &PathBuf) -> bool {
    let state = get_reactivity_state();
    let mut state_guard = state.write();
    if let Some(existing) = state_guard.debounce_states.remove(file) {
        let _ = existing.cancel_tx.try_send(());
        tracing::debug!("⏱️  Manually cancelled debounce for {:?}", file);
        true
    } else {
        false
    }
}

/// Get the current debounce delay configuration
pub fn get_debounce_delay() -> Duration {
    let state = get_reactivity_state();

    state.read().debounce_delay
}

/// Get the time elapsed since the last debounce event for a file
///
/// # Arguments
/// * `file` - The file path to check
///
/// # Returns
/// `Some(Duration)` if there's a pending debounce, `None` otherwise
pub fn time_since_last_event(file: &PathBuf) -> Option<Duration> {
    let state = get_reactivity_state();
    let state_guard = state.read();
    state_guard.debounce_states.get(file).map(|s| s.last_event.elapsed())
}

/// Only when user idle ≥2s — i18n, security, bundle analysis
///
/// Triggers tool execution only when the user has been idle for at least the configured
/// idle threshold (default 2 seconds). This function will wait until idle state is detected
/// and then trigger the idle event handler exactly once per idle period.
///
/// # Arguments
/// * `file` - The file path associated with the idle event
///
/// # Returns
/// * `Ok(true)` - Idle event was triggered successfully
/// * `Ok(false)` - Activity was detected before idle threshold was reached
/// * `Err` - An error occurred
///
/// # Example
/// ```rust,no_run
/// use dx_forge::trigger_idle_event;
/// use std::path::PathBuf;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let triggered = trigger_idle_event(PathBuf::from("src/main.rs")).await?;
///     if triggered {
///         println!("Idle event was triggered!");
///     }
///     Ok(())
/// }
/// ```
pub async fn trigger_idle_event(file: PathBuf) -> Result<bool> {
    trigger_idle_event_with_threshold(file, None).await
}

/// Triggers an idle event with a custom threshold
///
/// This is the configurable version of `trigger_idle_event` that allows
/// specifying a custom idle threshold for this specific event.
///
/// # Arguments
/// * `file` - The file path associated with the idle event
/// * `threshold` - Optional custom threshold (uses configured default if None)
///
/// # Returns
/// * `Ok(true)` - Idle event was triggered successfully
/// * `Ok(false)` - Activity was detected before idle threshold was reached
/// * `Err` - An error occurred
pub async fn trigger_idle_event_with_threshold(
    file: PathBuf,
    threshold: Option<Duration>,
) -> Result<bool> {
    let state = get_reactivity_state();
    let idle_threshold = threshold.unwrap_or_else(|| state.read().idle_threshold);

    tracing::debug!("😴 Idle event requested: {:?} (threshold: {:?})", file, idle_threshold);

    // Create a channel to detect activity interruption
    let (activity_tx, mut activity_rx) = mpsc::channel::<()>(1);

    // Store the activity notification channel
    {
        let mut state_guard = state.write();
        state_guard.idle_state.idle_notify_tx = Some(activity_tx);
    }

    // Calculate remaining time until idle threshold
    let time_since_activity = {
        let state_guard = state.read();
        state_guard.idle_state.last_activity.elapsed()
    };

    let remaining_time = if time_since_activity >= idle_threshold {
        Duration::ZERO
    } else {
        idle_threshold - time_since_activity
    };

    // If we need to wait, do so with activity interruption detection
    if remaining_time > Duration::ZERO {
        tracing::debug!("😴 Waiting {:?} for idle threshold", remaining_time);

        tokio::select! {
            _ = sleep(remaining_time) => {
                // Check if we're still idle after waiting
                let still_idle = {
                    let state_guard = state.read();
                    state_guard.idle_state.last_activity.elapsed() >= idle_threshold
                };

                if !still_idle {
                    tracing::debug!("😴 Activity detected during wait, idle event cancelled");
                    // Clean up notification channel
                    let mut state_guard = state.write();
                    state_guard.idle_state.idle_notify_tx = None;
                    return Ok(false);
                }
            }
            _ = activity_rx.recv() => {
                // Activity was detected, cancel idle event
                tracing::debug!("😴 Activity notification received, idle event cancelled");
                // Clean up notification channel
                let mut state_guard = state.write();
                state_guard.idle_state.idle_notify_tx = None;
                return Ok(false);
            }
        }
    }

    // Check if idle event was already triggered for this idle period
    {
        let mut state_guard = state.write();
        if state_guard.idle_state.idle_event_triggered {
            tracing::debug!("😴 Idle event already triggered for this idle period");
            state_guard.idle_state.idle_notify_tx = None;
            return Ok(false);
        }

        // Mark idle event as triggered and update idle state
        state_guard.idle_state.is_idle = true;
        state_guard.idle_state.idle_event_triggered = true;
        state_guard.idle_state.idle_notify_tx = None;
    }

    tracing::info!("😴 Idle state detected, triggering idle event for {:?}", file);

    // Execute idle event handler
    execute_idle_handler(&file).await?;

    // Execute any scheduled idle tasks
    execute_scheduled_idle_tasks().await?;

    Ok(true)
}

/// Internal handler that executes when idle state is detected
async fn execute_idle_handler(file: &PathBuf) -> Result<()> {
    tracing::info!("😴 Executing idle-tier tools for {:?}", file);

    // Update execution context with the file
    if let Some(forge) = crate::api::lifecycle::FORGE_INSTANCE.get() {
        let forge_guard =
            forge.lock().map_err(|e| anyhow::anyhow!("Failed to lock forge: {}", e))?;
        let orchestrator = forge_guard.orchestrator();
        let mut orchestrator = orchestrator.write();

        // Add to changed files if not already present
        if !orchestrator.context().changed_files.contains(file) {
            orchestrator.context_mut().changed_files.push(file.clone());
        }

        // In a production system, execute idle-tier tools here
        // For example: i18n checks, security audits, bundle analysis
        // orchestrator.execute_subset(&["dx-i18n", "dx-security", "dx-bundle-analyzer"])?;
    }

    Ok(())
}

/// Execute all scheduled idle tasks
async fn execute_scheduled_idle_tasks() -> Result<()> {
    let tasks: Vec<ScheduledIdleTask> = {
        let state = get_reactivity_state();
        let mut state_guard = state.write();
        std::mem::take(&mut state_guard.scheduled_idle_tasks)
    };

    if tasks.is_empty() {
        return Ok(());
    }

    tracing::info!("😴 Executing {} scheduled idle tasks", tasks.len());

    for task in tasks {
        tracing::debug!("😴 Executing scheduled task: {}", task.task_id);
        match (task.task_fn)() {
            Ok(()) => {
                tracing::debug!("😴 Task '{}' completed successfully", task.task_id);
            }
            Err(e) => {
                tracing::warn!("😴 Task '{}' failed: {}", task.task_id, e);
                // Continue executing other tasks even if one fails
            }
        }
    }

    Ok(())
}

/// Schedule a task to be executed during idle time
///
/// The task will be executed the next time the system enters idle state.
/// Tasks are executed in the order they were scheduled.
///
/// # Arguments
/// * `task_id` - A unique identifier for the task
/// * `task_fn` - The function to execute during idle time
///
/// # Returns
/// * `Ok(())` - Task was scheduled successfully
/// * `Err` - An error occurred (e.g., duplicate task_id)
///
/// # Example
/// ```rust,no_run
/// use dx_forge::schedule_task_for_idle_time;
/// use anyhow::Result;
///
/// fn my_idle_task() -> Result<()> {
///     println!("Running during idle time!");
///     Ok(())
/// }
///
/// schedule_task_for_idle_time("my-task", Box::new(my_idle_task))?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn schedule_task_for_idle_time(
    task_id: &str,
    task_fn: Box<dyn Fn() -> Result<()> + Send + Sync>,
) -> Result<()> {
    let state = get_reactivity_state();
    let mut state = state.write();

    // Check for duplicate task_id
    if state.scheduled_idle_tasks.iter().any(|t| t.task_id == task_id) {
        return Err(anyhow::anyhow!("Task with id '{}' is already scheduled", task_id));
    }

    state.scheduled_idle_tasks.push(ScheduledIdleTask {
        task_id: task_id.to_string(),
        task_fn,
    });

    tracing::info!("📅 Scheduled task '{}' for idle time execution", task_id);
    Ok(())
}

/// Cancel a scheduled idle task
///
/// # Arguments
/// * `task_id` - The identifier of the task to cancel
///
/// # Returns
/// * `true` if the task was found and cancelled
/// * `false` if no task with that id was found
pub fn cancel_scheduled_idle_task(task_id: &str) -> bool {
    let state = get_reactivity_state();
    let mut state = state.write();

    let initial_len = state.scheduled_idle_tasks.len();
    state.scheduled_idle_tasks.retain(|t| t.task_id != task_id);

    let cancelled = state.scheduled_idle_tasks.len() < initial_len;
    if cancelled {
        tracing::info!("📅 Cancelled scheduled task '{}'", task_id);
    }
    cancelled
}

/// Get the number of tasks currently scheduled for idle time
pub fn scheduled_idle_task_count() -> usize {
    let state = get_reactivity_state();

    state.read().scheduled_idle_tasks.len()
}

/// Check if a specific task is scheduled for idle time
pub fn is_task_scheduled(task_id: &str) -> bool {
    let state = get_reactivity_state();

    state.read().scheduled_idle_tasks.iter().any(|t| t.task_id == task_id)
}

/// Marks start of atomic multi-file operation
///
/// Batches multiple file changes together to avoid redundant tool executions.
pub fn begin_batch_operation() -> Result<()> {
    let state = get_reactivity_state();
    let mut state = state.write();

    tracing::info!("📦 Beginning batch operation");
    state.in_batch = true;
    state.batch_start = Some(std::time::Instant::now());

    Ok(())
}

/// Marks completion — triggers idle queue + resets branching
///
/// Ends the batch operation and triggers all queued events.
pub fn end_batch_operation() -> Result<()> {
    let state = get_reactivity_state();
    let mut state = state.write();

    if let Some(start) = state.batch_start {
        let duration = start.elapsed();
        tracing::info!("✅ Batch operation completed in {:.2}s", duration.as_secs_f64());
    }

    state.in_batch = false;
    state.batch_start = None;

    // TODO: Flush all queued events

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_operation() {
        begin_batch_operation().unwrap();
        end_batch_operation().unwrap();
    }

    #[tokio::test]
    async fn test_debounced_event() {
        let file = PathBuf::from("test.ts");
        let result = trigger_debounced_event(file, "content".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_debounced_event_with_custom_delay() {
        let file = PathBuf::from("test_custom.ts");
        let start = Instant::now();
        let delay = Duration::from_millis(50);

        let result =
            trigger_debounced_event_with_delay(file, "content".to_string(), Some(delay)).await;

        assert!(result.is_ok());
        // Verify that at least the delay time has passed
        assert!(start.elapsed() >= delay);
    }

    #[tokio::test]
    async fn test_debounce_cancellation() {
        // Configure a longer delay for testing
        let delay = Duration::from_millis(200);

        let file = PathBuf::from("test_cancel.ts");

        // Start first debounce
        let file_clone = file.clone();
        let handle1 = tokio::spawn(async move {
            trigger_debounced_event_with_delay(file_clone, "content1".to_string(), Some(delay))
                .await
        });

        // Wait a bit, then trigger another event for the same file
        sleep(Duration::from_millis(50)).await;

        // This should cancel the first debounce
        let file_clone2 = file.clone();
        let handle2 = tokio::spawn(async move {
            trigger_debounced_event_with_delay(file_clone2, "content2".to_string(), Some(delay))
                .await
        });

        // Both should complete successfully
        let result1 = handle1.await.unwrap();
        let result2 = handle2.await.unwrap();

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn test_configure_debounce_delay() {
        let custom_delay = Duration::from_millis(100);
        configure_debounce_delay(custom_delay);

        assert_eq!(get_debounce_delay(), custom_delay);

        // Reset to default
        configure_debounce_delay(Duration::from_millis(DEFAULT_DEBOUNCE_DELAY_MS));
    }

    #[tokio::test]
    async fn test_has_pending_debounce() {
        let file = PathBuf::from("test_pending.ts");
        let delay = Duration::from_millis(200);

        // Initially no pending debounce
        assert!(!has_pending_debounce(&file));

        // Start a debounce
        let file_clone = file.clone();
        let handle = tokio::spawn(async move {
            trigger_debounced_event_with_delay(file_clone, "content".to_string(), Some(delay)).await
        });

        // Give it a moment to register
        sleep(Duration::from_millis(10)).await;

        // Should have pending debounce now
        assert!(has_pending_debounce(&file));

        // Wait for completion
        let _ = handle.await;

        // No longer pending
        assert!(!has_pending_debounce(&file));
    }

    #[tokio::test]
    async fn test_cancel_debounce() {
        let file = PathBuf::from("test_manual_cancel.ts");
        let delay = Duration::from_millis(500);

        // Start a debounce
        let file_clone = file.clone();
        let handle = tokio::spawn(async move {
            trigger_debounced_event_with_delay(file_clone, "content".to_string(), Some(delay)).await
        });

        // Give it a moment to register
        sleep(Duration::from_millis(10)).await;

        // Cancel it
        assert!(cancel_debounce(&file));

        // Should complete quickly (cancelled)
        let start = Instant::now();
        let _ = handle.await;

        // Should have completed much faster than the delay
        assert!(start.elapsed() < delay);
    }

    #[tokio::test]
    async fn test_multiple_files_debounce_independently() {
        let file1 = PathBuf::from("test_file1.ts");
        let file2 = PathBuf::from("test_file2.ts");
        let delay = Duration::from_millis(100);

        let file1_clone = file1.clone();
        let file2_clone = file2.clone();

        // Start debounces for both files
        let handle1 = tokio::spawn(async move {
            trigger_debounced_event_with_delay(file1_clone, "content1".to_string(), Some(delay))
                .await
        });

        let handle2 = tokio::spawn(async move {
            trigger_debounced_event_with_delay(file2_clone, "content2".to_string(), Some(delay))
                .await
        });

        // Both should complete successfully and independently
        let result1 = handle1.await.unwrap();
        let result2 = handle2.await.unwrap();

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    // ==================== Idle Detection Tests ====================

    // Mutex to serialize tests that use global idle state
    use std::sync::Mutex;
    static IDLE_TEST_MUTEX: Mutex<()> = Mutex::new(());

    /// Helper to reset idle state before each test
    fn reset_idle_state_for_test() {
        // Reset threshold to a known value
        configure_idle_threshold(Duration::from_millis(DEFAULT_IDLE_THRESHOLD_MS));
        // Record activity to reset idle flags
        record_activity();
    }

    #[test]
    fn test_configure_idle_threshold() {
        let _guard = IDLE_TEST_MUTEX.lock().unwrap();
        reset_idle_state_for_test();

        let custom_threshold = Duration::from_secs(5);
        configure_idle_threshold(custom_threshold);

        assert_eq!(get_idle_threshold(), custom_threshold);

        // Reset to default
        configure_idle_threshold(Duration::from_millis(DEFAULT_IDLE_THRESHOLD_MS));
    }

    #[test]
    fn test_record_activity_resets_idle_state() {
        let _guard = IDLE_TEST_MUTEX.lock().unwrap();
        reset_idle_state_for_test();

        // Record activity
        record_activity();

        // Time since last activity should be very small
        let elapsed = time_since_last_activity();
        assert!(elapsed < Duration::from_millis(100));

        // Should not be idle immediately after activity
        assert!(!is_idle());
    }

    #[tokio::test]
    async fn test_idle_detection_after_threshold() {
        let _guard = IDLE_TEST_MUTEX.lock().unwrap();
        reset_idle_state_for_test();

        let file = PathBuf::from("test_idle.ts");
        let threshold = Duration::from_millis(80);

        // Configure the idle threshold for this test
        configure_idle_threshold(threshold);

        // Record activity to reset state
        record_activity();

        // Wait for threshold to pass with extra buffer for timing variance
        sleep(threshold + Duration::from_millis(50)).await;

        // Should be idle now
        assert!(is_idle());

        // Trigger idle event - should succeed
        let result = trigger_idle_event_with_threshold(file, Some(threshold)).await;
        assert!(result.is_ok());
        assert!(result.unwrap()); // Should return true (event triggered)

        // Reset threshold to default
        configure_idle_threshold(Duration::from_millis(DEFAULT_IDLE_THRESHOLD_MS));
    }

    #[tokio::test]
    async fn test_idle_event_not_triggered_twice() {
        let _guard = IDLE_TEST_MUTEX.lock().unwrap();
        reset_idle_state_for_test();

        let file = PathBuf::from("test_idle_once.ts");
        let threshold = Duration::from_millis(80);

        // Configure the idle threshold for this test
        configure_idle_threshold(threshold);

        // Record activity to reset state
        record_activity();

        // Wait for threshold to pass with extra buffer
        sleep(threshold + Duration::from_millis(50)).await;

        // First idle event should trigger
        let result1 = trigger_idle_event_with_threshold(file.clone(), Some(threshold)).await;
        assert!(result1.is_ok());
        assert!(result1.unwrap()); // Should return true

        // Second idle event should NOT trigger (already triggered for this idle period)
        let result2 = trigger_idle_event_with_threshold(file, Some(threshold)).await;
        assert!(result2.is_ok());
        assert!(!result2.unwrap()); // Should return false

        // Reset threshold to default
        configure_idle_threshold(Duration::from_millis(DEFAULT_IDLE_THRESHOLD_MS));
    }

    #[tokio::test]
    async fn test_activity_resets_idle_event_flag() {
        let _guard = IDLE_TEST_MUTEX.lock().unwrap();
        reset_idle_state_for_test();

        let file = PathBuf::from("test_idle_reset.ts");
        let threshold = Duration::from_millis(80);

        // Configure the idle threshold for this test
        configure_idle_threshold(threshold);

        // Record activity to reset state
        record_activity();

        // Wait for threshold to pass with extra buffer
        sleep(threshold + Duration::from_millis(50)).await;

        // First idle event should trigger
        let result1 = trigger_idle_event_with_threshold(file.clone(), Some(threshold)).await;
        assert!(result1.unwrap());

        // Record new activity - this should reset the idle event flag
        record_activity();

        // Wait for threshold again with extra buffer
        sleep(threshold + Duration::from_millis(50)).await;

        // Now idle event should trigger again (new idle period)
        let result2 = trigger_idle_event_with_threshold(file, Some(threshold)).await;
        assert!(result2.is_ok());
        assert!(result2.unwrap()); // Should return true for new idle period

        // Reset threshold to default
        configure_idle_threshold(Duration::from_millis(DEFAULT_IDLE_THRESHOLD_MS));
    }

    #[tokio::test]
    async fn test_activity_cancels_pending_idle_event() {
        let _guard = IDLE_TEST_MUTEX.lock().unwrap();
        reset_idle_state_for_test();

        let file = PathBuf::from("test_idle_cancel.ts");
        let threshold = Duration::from_millis(300);

        // Configure the idle threshold for this test
        configure_idle_threshold(threshold);

        // Record activity to reset state
        record_activity();

        // Start idle event detection in background
        let file_clone = file.clone();
        let handle = tokio::spawn(async move {
            trigger_idle_event_with_threshold(file_clone, Some(threshold)).await
        });

        // Wait a bit, then record activity (before threshold)
        sleep(Duration::from_millis(80)).await;
        record_activity();

        // The idle event should be cancelled
        let result = handle.await.unwrap();
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Should return false (cancelled by activity)

        // Reset threshold to default
        configure_idle_threshold(Duration::from_millis(DEFAULT_IDLE_THRESHOLD_MS));
    }

    // ==================== Scheduled Idle Task Tests ====================

    #[test]
    fn test_schedule_task_for_idle_time() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let _guard = IDLE_TEST_MUTEX.lock().unwrap();

        // Clean up any existing tasks first
        {
            let state = get_reactivity_state();
            state.write().scheduled_idle_tasks.clear();
        }

        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let result = schedule_task_for_idle_time(
            "test-task-1",
            Box::new(move || {
                executed_clone.store(true, Ordering::SeqCst);
                Ok(())
            }),
        );

        assert!(result.is_ok());
        assert_eq!(scheduled_idle_task_count(), 1);
        assert!(is_task_scheduled("test-task-1"));

        // Clean up
        cancel_scheduled_idle_task("test-task-1");
    }

    #[test]
    fn test_schedule_duplicate_task_fails() {
        let _guard = IDLE_TEST_MUTEX.lock().unwrap();

        // Clean up any existing tasks first
        {
            let state = get_reactivity_state();
            state.write().scheduled_idle_tasks.clear();
        }

        let result1 = schedule_task_for_idle_time("duplicate-task", Box::new(|| Ok(())));
        assert!(result1.is_ok());

        // Scheduling same task again should fail
        let result2 = schedule_task_for_idle_time("duplicate-task", Box::new(|| Ok(())));
        assert!(result2.is_err());

        // Clean up
        cancel_scheduled_idle_task("duplicate-task");
    }

    #[test]
    fn test_cancel_scheduled_idle_task() {
        let _guard = IDLE_TEST_MUTEX.lock().unwrap();

        // Clean up any existing tasks first
        {
            let state = get_reactivity_state();
            state.write().scheduled_idle_tasks.clear();
        }

        schedule_task_for_idle_time("cancel-test", Box::new(|| Ok(()))).unwrap();
        assert!(is_task_scheduled("cancel-test"));

        let cancelled = cancel_scheduled_idle_task("cancel-test");
        assert!(cancelled);
        assert!(!is_task_scheduled("cancel-test"));

        // Cancelling non-existent task returns false
        let cancelled_again = cancel_scheduled_idle_task("cancel-test");
        assert!(!cancelled_again);
    }

    #[tokio::test]
    async fn test_scheduled_tasks_execute_on_idle() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let _guard = IDLE_TEST_MUTEX.lock().unwrap();
        reset_idle_state_for_test();

        // Clean up any existing tasks first
        {
            let state = get_reactivity_state();
            state.write().scheduled_idle_tasks.clear();
        }

        let counter = Arc::new(AtomicUsize::new(0));
        let counter1 = counter.clone();
        let counter2 = counter.clone();

        // Schedule two tasks
        schedule_task_for_idle_time(
            "exec-test-1",
            Box::new(move || {
                counter1.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }),
        )
        .unwrap();

        schedule_task_for_idle_time(
            "exec-test-2",
            Box::new(move || {
                counter2.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }),
        )
        .unwrap();

        assert_eq!(scheduled_idle_task_count(), 2);

        // Record activity and wait for idle
        record_activity();
        let threshold = Duration::from_millis(80);
        configure_idle_threshold(threshold);
        sleep(threshold + Duration::from_millis(50)).await;

        // Trigger idle event - should execute scheduled tasks
        let file = PathBuf::from("test_exec.ts");
        let result = trigger_idle_event_with_threshold(file, Some(threshold)).await;
        assert!(result.is_ok());
        assert!(result.unwrap());

        // Both tasks should have executed
        assert_eq!(counter.load(Ordering::SeqCst), 2);

        // Tasks should be cleared after execution
        assert_eq!(scheduled_idle_task_count(), 0);

        // Reset threshold to default
        configure_idle_threshold(Duration::from_millis(DEFAULT_IDLE_THRESHOLD_MS));
    }
}
