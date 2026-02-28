use crate::ui::spinner::Spinner;
use std::time::Duration;

/// Run an async task with a spinner
pub async fn with_spinner<F, T>(message: impl Into<String>, task: F) -> T
where
    F: std::future::Future<Output = T>,
{
    let spinner = Spinner::dots(message);
    let result = task.await;
    spinner.finish();
    result
}

/// Run an async task with a spinner and success message
pub async fn with_spinner_success<F, T>(
    message: impl Into<String>,
    success: impl Into<String>,
    task: F,
) -> T
where
    F: std::future::Future<Output = T>,
{
    let spinner = Spinner::dots(message);
    let result = task.await;
    spinner.success(success);
    result
}

/// Simulate a task with delay and show success
pub async fn simulate_task(message: impl Into<String>, success: impl Into<String>, delay_ms: u64) {
    let spinner = Spinner::dots(message);
    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    spinner.success(success);
}

/// Run multiple tasks sequentially with spinners
pub async fn run_tasks(tasks: Vec<(&str, &str, u64)>) {
    for (message, success, delay) in tasks {
        simulate_task(message, success, delay).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_with_spinner() {
        let result = with_spinner("Testing...", async { 42 }).await;
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_with_spinner_success() {
        let result = with_spinner_success("Testing...", "Done!", async { 42 }).await;
        assert_eq!(result, 42);
    }
}
