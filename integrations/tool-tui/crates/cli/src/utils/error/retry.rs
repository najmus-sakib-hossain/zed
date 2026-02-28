//! Retry logic with exponential backoff

use super::context::{EnhancedError, ErrorContext};
use super::types::DxError;

/// Execute an async operation with retry logic and exponential backoff
pub async fn with_retry<T, F, Fut>(
    operation_name: &str,
    max_retries: u32,
    mut operation: F,
) -> Result<T, EnhancedError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, DxError>>,
{
    let context = ErrorContext::new(operation_name);
    let mut retry_count = 0;

    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(error) => {
                let enhanced =
                    EnhancedError::with_retries(error, context.clone(), retry_count, max_retries);

                if enhanced.should_retry() {
                    let delay = enhanced.next_retry_delay();
                    tokio::time::sleep(delay).await;
                    retry_count += 1;
                } else {
                    return Err(EnhancedError::with_retries(
                        enhanced.error,
                        enhanced.context,
                        retry_count,
                        max_retries,
                    ));
                }
            }
        }
    }
}

/// Synchronous version of with_retry for non-async contexts
#[allow(clippy::result_large_err)]
pub fn with_retry_sync<T, F>(
    operation_name: &str,
    max_retries: u32,
    mut operation: F,
) -> Result<T, EnhancedError>
where
    F: FnMut() -> Result<T, DxError>,
{
    let context = ErrorContext::new(operation_name);
    let mut retry_count = 0;

    loop {
        match operation() {
            Ok(result) => return Ok(result),
            Err(error) => {
                let enhanced =
                    EnhancedError::with_retries(error, context.clone(), retry_count, max_retries);

                if enhanced.should_retry() {
                    let delay = enhanced.next_retry_delay();
                    std::thread::sleep(delay);
                    retry_count += 1;
                } else {
                    return Err(EnhancedError::with_retries(
                        enhanced.error,
                        enhanced.context,
                        retry_count,
                        max_retries,
                    ));
                }
            }
        }
    }
}
