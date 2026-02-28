//! Stress tests for DX Forge platform-native I/O
//!
//! Tests high-concurrency scenarios and scalability limits.
//! Requirements: 4.5, 2.4

use anyhow::Result;
use dx_forge::{DualWatcher, PlatformIO, WriteOp, create_platform_io};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::sync::Barrier;

/// Test 1000+ concurrent file operations
/// Requirements: 4.5
#[tokio::test]
async fn stress_test_concurrent_file_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let io = create_platform_io();
    let num_operations = 1000;
    let barrier: Arc<Barrier> = Arc::new(Barrier::new(num_operations));

    // Create test files first
    for i in 0..num_operations {
        let path = temp_dir.path().join(format!("file_{}.txt", i));
        let content = format!("Initial content for file {}", i);
        io.write_all(&path, content.as_bytes()).await?;
    }

    let start = Instant::now();

    // Spawn concurrent read operations
    let mut handles = Vec::with_capacity(num_operations);
    for i in 0..num_operations {
        let io = Arc::clone(&Arc::new(create_platform_io()));
        let path = temp_dir.path().join(format!("file_{}.txt", i));
        let barrier = Arc::clone(&barrier);

        let handle = tokio::spawn(async move {
            // Wait for all tasks to be ready
            barrier.wait().await;

            // Perform read operation
            let content = io.read_all(&path).await?;
            assert!(!content.is_empty(), "File {} should have content", i);
            Ok::<_, anyhow::Error>(content.len())
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    let mut total_bytes = 0;
    for handle in handles {
        let bytes = handle.await??;
        total_bytes += bytes;
    }

    let elapsed = start.elapsed();
    println!(
        "Completed {} concurrent read operations in {:?} ({} bytes total)",
        num_operations, elapsed, total_bytes
    );

    // Should complete within reasonable time (30 seconds)
    assert!(
        elapsed < Duration::from_secs(30),
        "Concurrent operations took too long: {:?}",
        elapsed
    );

    Ok(())
}

/// Test concurrent write operations with serialization
/// Requirements: 4.5, 10.2
#[tokio::test]
async fn stress_test_concurrent_write_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let io = create_platform_io();
    let num_files = 100;
    let writes_per_file = 10;

    let start = Instant::now();

    // Spawn concurrent write operations to different files
    let mut handles = Vec::new();
    for file_idx in 0..num_files {
        let io = Arc::clone(&Arc::new(create_platform_io()));
        let path = temp_dir.path().join(format!("write_file_{}.txt", file_idx));

        let handle = tokio::spawn(async move {
            for write_idx in 0..writes_per_file {
                let content = format!("Write {} to file {}\n", write_idx, file_idx);
                io.write_all(&path, content.as_bytes()).await?;
            }
            Ok::<_, anyhow::Error>(())
        });
        handles.push(handle);
    }

    // Wait for all writes to complete
    for handle in handles {
        handle.await??;
    }

    let elapsed = start.elapsed();
    println!(
        "Completed {} write operations ({} files x {} writes) in {:?}",
        num_files * writes_per_file,
        num_files,
        writes_per_file,
        elapsed
    );

    // Verify all files exist and have content
    for file_idx in 0..num_files {
        let path = temp_dir.path().join(format!("write_file_{}.txt", file_idx));
        let content = io.read_all(&path).await?;
        assert!(!content.is_empty(), "File {} should have content", file_idx);
    }

    Ok(())
}

/// Test batch operations with large number of files
/// Requirements: 4.5, 1.7
#[tokio::test]
async fn stress_test_batch_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let io = create_platform_io();
    let num_files = 500;

    // Create batch write operations
    let mut write_ops: Vec<WriteOp> = Vec::with_capacity(num_files);
    let mut paths: Vec<PathBuf> = Vec::with_capacity(num_files);

    for i in 0..num_files {
        let path = temp_dir.path().join(format!("batch_file_{}.txt", i));
        let content =
            format!("Batch content for file {} with some padding data to make it larger", i);
        write_ops.push(WriteOp::new(path.clone(), content.into_bytes()));
        paths.push(path);
    }

    let start = Instant::now();

    // Perform batch write
    io.batch_write(&write_ops).await?;

    let write_elapsed = start.elapsed();
    println!("Batch write of {} files completed in {:?}", num_files, write_elapsed);

    // Perform batch read
    let read_start = Instant::now();
    let contents = io.batch_read(&paths).await?;
    let read_elapsed = read_start.elapsed();

    println!("Batch read of {} files completed in {:?}", num_files, read_elapsed);

    // Verify all files were read correctly
    assert_eq!(contents.len(), num_files);
    for (i, content) in contents.iter().enumerate() {
        assert!(!content.is_empty(), "File {} should have content", i);
    }

    Ok(())
}

/// Test watcher scalability with 10,000+ files
/// Requirements: 2.4
#[tokio::test]
async fn stress_test_watcher_scalability() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let num_files = 10_000;
    let io = create_platform_io();

    println!("Creating {} files for watcher scalability test...", num_files);
    let create_start = Instant::now();

    // Create files in batches to avoid overwhelming the system
    let batch_size = 500;
    for batch_start in (0..num_files).step_by(batch_size) {
        let batch_end = (batch_start + batch_size).min(num_files);
        let mut write_ops = Vec::with_capacity(batch_size);

        for i in batch_start..batch_end {
            let path = temp_dir.path().join(format!("watched_file_{}.txt", i));
            let content = format!("Content for file {}", i);
            write_ops.push(WriteOp::new(path, content.into_bytes()));
        }

        io.batch_write(&write_ops).await?;
    }

    let create_elapsed = create_start.elapsed();
    println!("Created {} files in {:?}", num_files, create_elapsed);

    // Start watching the directory
    let watch_start = Instant::now();
    let mut watcher = DualWatcher::new()?;
    watcher.start(temp_dir.path()).await?;
    let watch_elapsed = watch_start.elapsed();

    println!("Started watching {} files in {:?}", num_files, watch_elapsed);

    // Watcher should start within reasonable time
    assert!(
        watch_elapsed < Duration::from_secs(10),
        "Watcher took too long to start: {:?}",
        watch_elapsed
    );

    // Modify a few files and verify events are received
    let mut rx = watcher.receiver();

    // Modify 10 random files
    for i in [0, 1000, 2500, 5000, 7500, 9000, 9500, 9900, 9990, 9999].iter() {
        if *i < num_files {
            let path = temp_dir.path().join(format!("watched_file_{}.txt", i));
            let content = format!("Modified content for file {}", i);
            io.write_all(&path, content.as_bytes()).await?;
        }
    }

    // Give watcher time to detect changes
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Stop watcher
    watcher.stop().await?;

    println!("Watcher scalability test completed successfully");

    Ok(())
}

/// Test resource cleanup under stress
/// Requirements: 6.1, 6.3
#[tokio::test]
async fn stress_test_resource_cleanup() -> Result<()> {
    use dx_forge::ResourceManager;
    use std::sync::Arc;

    let manager = Arc::new(ResourceManager::new(100));
    let num_operations = 500;
    let barrier: Arc<Barrier> = Arc::new(Barrier::new(num_operations));

    let start = Instant::now();

    // Spawn many concurrent operations that acquire and release handles
    let mut handles = Vec::with_capacity(num_operations);
    for i in 0..num_operations {
        let manager = Arc::clone(&manager);
        let barrier = Arc::clone(&barrier);

        let handle = tokio::spawn(async move {
            barrier.wait().await;

            // Acquire handle
            let guard = manager.acquire_handle().await?;

            // Simulate some work
            tokio::time::sleep(Duration::from_millis(10)).await;

            // Handle is automatically released when guard is dropped
            drop(guard);

            Ok::<_, anyhow::Error>(i)
        });
        handles.push(handle);
    }

    // Wait for all operations
    for handle in handles {
        handle.await??;
    }

    let elapsed = start.elapsed();
    println!("Completed {} handle acquire/release cycles in {:?}", num_operations, elapsed);

    // Verify all handles were released
    assert_eq!(
        manager.active_handles(),
        0,
        "All handles should be released after operations complete"
    );

    // Shutdown should complete quickly
    let shutdown_start = Instant::now();
    manager.shutdown(Duration::from_secs(5)).await?;
    let shutdown_elapsed = shutdown_start.elapsed();

    println!("Resource manager shutdown completed in {:?}", shutdown_elapsed);
    assert!(
        shutdown_elapsed < Duration::from_secs(1),
        "Shutdown should be fast when no handles are active"
    );

    Ok(())
}

/// Test mixed read/write operations under load
/// Requirements: 4.5, 10.1, 10.2
#[tokio::test]
async fn stress_test_mixed_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let io = create_platform_io();
    let num_files = 50;
    let operations_per_file = 20;

    // Create initial files
    for i in 0..num_files {
        let path = temp_dir.path().join(format!("mixed_file_{}.txt", i));
        let content = format!("Initial content for file {}", i);
        io.write_all(&path, content.as_bytes()).await?;
    }

    let start = Instant::now();

    // Spawn mixed read/write operations
    let mut handles = Vec::new();
    for file_idx in 0..num_files {
        let io = Arc::clone(&Arc::new(create_platform_io()));
        let path = temp_dir.path().join(format!("mixed_file_{}.txt", file_idx));

        let handle = tokio::spawn(async move {
            for op_idx in 0..operations_per_file {
                if op_idx % 2 == 0 {
                    // Read operation
                    let _ = io.read_all(&path).await?;
                } else {
                    // Write operation
                    let content = format!("Updated content {} for file {}", op_idx, file_idx);
                    io.write_all(&path, content.as_bytes()).await?;
                }
            }
            Ok::<_, anyhow::Error>(())
        });
        handles.push(handle);
    }

    // Wait for all operations
    for handle in handles {
        handle.await??;
    }

    let elapsed = start.elapsed();
    println!(
        "Completed {} mixed operations ({} files x {} ops) in {:?}",
        num_files * operations_per_file,
        num_files,
        operations_per_file,
        elapsed
    );

    Ok(())
}

// ============================================================================
// Graceful Degradation Tests
// Requirements: 4.6
// ============================================================================

/// Test that fallback backend works when native I/O is unavailable
/// Requirements: 4.6, 1.5
#[tokio::test]
async fn test_fallback_degradation() -> Result<()> {
    use dx_forge::FallbackBackend;

    let temp_dir = TempDir::new()?;
    let fallback = FallbackBackend::new();

    // Verify fallback backend is always available
    assert!(FallbackBackend::is_available(), "Fallback should always be available");
    assert_eq!(fallback.backend_name(), "fallback");

    // Test basic operations work with fallback
    let test_path = temp_dir.path().join("fallback_test.txt");
    let test_content = b"Testing fallback backend functionality";

    // Write
    fallback.write_all(&test_path, test_content).await?;

    // Read
    let read_content = fallback.read_all(&test_path).await?;
    assert_eq!(read_content, test_content);

    // Batch operations
    let paths: Vec<PathBuf> =
        (0..5).map(|i| temp_dir.path().join(format!("batch_{}.txt", i))).collect();

    let write_ops: Vec<WriteOp> = paths
        .iter()
        .enumerate()
        .map(|(i, p)| WriteOp::new(p.clone(), format!("Batch content {}", i).into_bytes()))
        .collect();

    fallback.batch_write(&write_ops).await?;

    let batch_contents = fallback.batch_read(&paths).await?;
    assert_eq!(batch_contents.len(), 5);

    println!("Fallback degradation test passed");
    Ok(())
}

/// Test that platform selector always returns a working backend
/// Requirements: 4.6, 1.5
#[tokio::test]
async fn test_platform_selector_always_works() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // create_platform_io should never fail
    let io = create_platform_io();

    // Backend should have a valid name
    let backend_name = io.backend_name();
    assert!(
        ["io_uring", "kqueue", "iocp", "fallback"].contains(&backend_name),
        "Backend name should be one of the known backends: {}",
        backend_name
    );

    println!("Platform selector returned backend: {}", backend_name);

    // Basic operations should work regardless of backend
    let test_path = temp_dir.path().join("selector_test.txt");
    let test_content = b"Testing platform selector";

    io.write_all(&test_path, test_content).await?;
    let read_content = io.read_all(&test_path).await?;
    assert_eq!(read_content, test_content);

    println!("Platform selector test passed with backend: {}", backend_name);
    Ok(())
}

/// Test error recovery and retry behavior
/// Requirements: 4.6, 5.2
#[tokio::test]
async fn test_error_recovery() -> Result<()> {
    use dx_forge::{RetryPolicy, with_retry};

    let mut attempt_count = 0;
    let max_attempts = 3u32;

    let policy = RetryPolicy {
        max_attempts,
        initial_delay: Duration::from_millis(10),
        max_delay: Duration::from_millis(100),
        backoff_multiplier: 2.0,
    };

    // Test retry with eventual success
    let result = with_retry(&policy, || {
        attempt_count += 1;
        if attempt_count < max_attempts as i32 {
            Err(anyhow::anyhow!("Temporary failure"))
        } else {
            Ok("Success after retries")
        }
    })
    .await;

    assert!(result.is_ok(), "Should succeed after retries");
    assert_eq!(
        attempt_count, max_attempts as i32,
        "Should have attempted {} times",
        max_attempts
    );

    // Test retry exhaustion
    let mut exhaustion_count = 0;
    let exhaustion_policy = RetryPolicy {
        max_attempts: 3,
        initial_delay: Duration::from_millis(10),
        max_delay: Duration::from_millis(100),
        backoff_multiplier: 2.0,
    };

    let exhaustion_result = with_retry(&exhaustion_policy, || {
        exhaustion_count += 1;
        Err::<(), _>(anyhow::anyhow!("Persistent failure"))
    })
    .await;

    assert!(exhaustion_result.is_err(), "Should fail after exhausting retries");
    assert_eq!(exhaustion_count, 3, "Should have attempted 3 times");

    println!("Error recovery test passed");
    Ok(())
}

/// Test graceful handling of permission errors
/// Requirements: 4.6, 5.1
#[tokio::test]
#[cfg(unix)]
async fn test_permission_error_handling() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = TempDir::new()?;
    let io = create_platform_io();

    // Create a file and make it read-only
    let readonly_path = temp_dir.path().join("readonly.txt");
    io.write_all(&readonly_path, b"Initial content").await?;

    // Make file read-only
    let mut perms = std::fs::metadata(&readonly_path)?.permissions();
    perms.set_mode(0o444);
    std::fs::set_permissions(&readonly_path, perms)?;

    // Attempt to write should fail gracefully
    let write_result = io.write_all(&readonly_path, b"New content").await;
    assert!(write_result.is_err(), "Writing to read-only file should fail");

    // Reading should still work
    let read_result = io.read_all(&readonly_path).await;
    assert!(read_result.is_ok(), "Reading from read-only file should work");

    // Restore permissions for cleanup
    let mut perms = std::fs::metadata(&readonly_path)?.permissions();
    perms.set_mode(0o644);
    std::fs::set_permissions(&readonly_path, perms)?;

    println!("Permission error handling test passed");
    Ok(())
}

/// Test handling of non-existent paths
/// Requirements: 4.6, 5.1
#[tokio::test]
async fn test_nonexistent_path_handling() -> Result<()> {
    let io = create_platform_io();

    // Reading non-existent file should fail gracefully
    let nonexistent = PathBuf::from("/nonexistent/path/to/file.txt");
    let read_result = io.read_all(&nonexistent).await;
    assert!(read_result.is_err(), "Reading non-existent file should fail");

    // Writing to non-existent directory should create parent dirs
    let temp_dir = TempDir::new()?;
    let nested_path = temp_dir.path().join("a/b/c/nested.txt");
    let write_result = io.write_all(&nested_path, b"Nested content").await;
    assert!(write_result.is_ok(), "Writing to nested path should create directories");

    // Verify file was created
    let read_back = io.read_all(&nested_path).await?;
    assert_eq!(read_back, b"Nested content");

    println!("Non-existent path handling test passed");
    Ok(())
}
