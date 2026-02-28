//! Platform I/O Integration Tests
//!
//! These tests verify that platform-native I/O backends work correctly
//! by performing actual file I/O operations on each platform.
//!
//! - Linux: Tests io_uring backend (kernel 5.1+)
//! - macOS: Tests kqueue backend
//! - Windows: Tests IOCP backend
//!
//! **Validates: Requirements 11.1, 11.2, 11.3, 11.5**

use anyhow::Result;
use dx_forge::{
    PlatformIO, WriteOp, create_platform_io, create_platform_io_with_fallback_tracking,
};
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// Common Integration Tests (Run on All Platforms)
// ============================================================================

/// Test that the platform I/O backend is correctly selected for the current platform.
/// **Validates: Requirements 11.1, 11.2, 11.3**
#[tokio::test]
async fn test_correct_backend_selected() -> Result<()> {
    let (io, did_fallback) = create_platform_io_with_fallback_tracking();
    let backend_name = io.backend_name();

    println!("Platform I/O backend selected: {}", backend_name);
    println!("Did fallback occur: {}", did_fallback);

    // Verify backend name is valid
    assert!(
        ["io_uring", "kqueue", "iocp", "fallback"].contains(&backend_name),
        "Backend name should be one of the known backends: {}",
        backend_name
    );

    // Platform-specific assertions
    #[cfg(target_os = "linux")]
    {
        // On Linux, we expect either io_uring or fallback
        assert!(
            backend_name == "io_uring" || backend_name == "fallback",
            "On Linux, expected io_uring or fallback, got: {}",
            backend_name
        );
        if backend_name == "io_uring" {
            println!("✓ io_uring backend is active on Linux");
        } else {
            println!("⚠ Fallback backend is active on Linux (io_uring not available)");
        }
    }

    #[cfg(target_os = "macos")]
    {
        // On macOS, we expect either kqueue or fallback
        assert!(
            backend_name == "kqueue" || backend_name == "fallback",
            "On macOS, expected kqueue or fallback, got: {}",
            backend_name
        );
        if backend_name == "kqueue" {
            println!("✓ kqueue backend is active on macOS");
        } else {
            println!("⚠ Fallback backend is active on macOS (kqueue not available)");
        }
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, we expect either iocp or fallback
        assert!(
            backend_name == "iocp" || backend_name == "fallback",
            "On Windows, expected iocp or fallback, got: {}",
            backend_name
        );
        if backend_name == "iocp" {
            println!("✓ IOCP backend is active on Windows");
        } else {
            println!("⚠ Fallback backend is active on Windows (IOCP not available)");
        }
    }

    Ok(())
}

/// Test actual file read/write operations using the platform backend.
/// **Validates: Requirements 11.5**
#[tokio::test]
async fn test_actual_file_read_write() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let io = create_platform_io();
    let backend_name = io.backend_name();

    println!("Testing actual I/O operations with backend: {}", backend_name);

    // Test 1: Write and read a simple file
    let simple_path = temp_dir.path().join("simple.txt");
    let simple_content = b"Hello, Platform I/O!";

    io.write_all(&simple_path, simple_content).await?;
    let read_content = io.read_all(&simple_path).await?;

    assert_eq!(
        read_content, simple_content,
        "Simple file content should match after round-trip"
    );
    println!("✓ Simple file read/write passed");

    // Test 2: Write and read binary data
    let binary_path = temp_dir.path().join("binary.bin");
    let binary_content: Vec<u8> = (0..256).map(|i| i as u8).collect();

    io.write_all(&binary_path, &binary_content).await?;
    let read_binary = io.read_all(&binary_path).await?;

    assert_eq!(read_binary, binary_content, "Binary file content should match after round-trip");
    println!("✓ Binary file read/write passed");

    // Test 3: Write and read a larger file (1MB)
    let large_path = temp_dir.path().join("large.bin");
    let large_content: Vec<u8> = (0..1_000_000).map(|i| (i % 256) as u8).collect();

    io.write_all(&large_path, &large_content).await?;
    let read_large = io.read_all(&large_path).await?;

    assert_eq!(read_large.len(), large_content.len(), "Large file size should match");
    assert_eq!(read_large, large_content, "Large file content should match after round-trip");
    println!("✓ Large file (1MB) read/write passed");

    // Test 4: Write and read an empty file
    let empty_path = temp_dir.path().join("empty.txt");
    let empty_content: Vec<u8> = vec![];

    io.write_all(&empty_path, &empty_content).await?;
    let read_empty = io.read_all(&empty_path).await?;

    assert!(read_empty.is_empty(), "Empty file should remain empty");
    println!("✓ Empty file read/write passed");

    // Test 5: Write to nested directory (auto-create parents)
    let nested_path = temp_dir.path().join("a/b/c/nested.txt");
    let nested_content = b"Nested file content";

    io.write_all(&nested_path, nested_content).await?;
    let read_nested = io.read_all(&nested_path).await?;

    assert_eq!(read_nested, nested_content, "Nested file content should match");
    println!("✓ Nested directory creation and file write passed");

    println!("All actual I/O operations passed with backend: {}", backend_name);
    Ok(())
}

/// Test batch read/write operations using the platform backend.
/// **Validates: Requirements 11.5**
#[tokio::test]
async fn test_batch_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let io = create_platform_io();
    let backend_name = io.backend_name();

    println!("Testing batch operations with backend: {}", backend_name);

    // Create batch write operations
    let num_files = 50;
    let mut write_ops: Vec<WriteOp> = Vec::with_capacity(num_files);
    let mut paths: Vec<PathBuf> = Vec::with_capacity(num_files);

    for i in 0..num_files {
        let path = temp_dir.path().join(format!("batch_file_{}.txt", i));
        let content = format!(
            "Batch content for file {} with some additional data to make it more realistic",
            i
        );
        write_ops.push(WriteOp::new(path.clone(), content.into_bytes()));
        paths.push(path);
    }

    // Perform batch write
    io.batch_write(&write_ops).await?;
    println!("✓ Batch write of {} files completed", num_files);

    // Verify all files exist
    for path in &paths {
        assert!(path.exists(), "File should exist after batch write: {:?}", path);
    }

    // Perform batch read
    let contents = io.batch_read(&paths).await?;
    println!("✓ Batch read of {} files completed", num_files);

    // Verify all contents match
    assert_eq!(contents.len(), num_files, "Should read all files");
    for (i, content) in contents.iter().enumerate() {
        let expected = format!(
            "Batch content for file {} with some additional data to make it more realistic",
            i
        );
        assert_eq!(content, expected.as_bytes(), "File {} content should match", i);
    }
    println!("✓ All batch file contents verified");

    println!("Batch operations test passed with backend: {}", backend_name);
    Ok(())
}

/// Test partial read operations using the platform backend.
/// **Validates: Requirements 11.5**
#[tokio::test]
async fn test_partial_read() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let io = create_platform_io();
    let backend_name = io.backend_name();

    println!("Testing partial read with backend: {}", backend_name);

    // Create a file with known content
    let file_path = temp_dir.path().join("partial_read.txt");
    let content = b"0123456789ABCDEFGHIJ";
    io.write_all(&file_path, content).await?;

    // Read into a smaller buffer
    let mut buf = vec![0u8; 10];
    let bytes_read = io.read(&file_path, &mut buf).await?;

    assert_eq!(bytes_read, 10, "Should read 10 bytes");
    assert_eq!(&buf[..bytes_read], b"0123456789", "Partial read content should match");
    println!("✓ Partial read test passed");

    Ok(())
}

/// Test partial write operations using the platform backend.
/// **Validates: Requirements 11.5**
#[tokio::test]
async fn test_partial_write() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let io = create_platform_io();
    let backend_name = io.backend_name();

    println!("Testing partial write with backend: {}", backend_name);

    // Write partial content
    let file_path = temp_dir.path().join("partial_write.txt");
    let content = b"Hello, World!";
    let bytes_written = io.write(&file_path, content).await?;

    assert!(bytes_written > 0, "Should write some bytes");
    println!("✓ Wrote {} bytes", bytes_written);

    // Read back and verify
    let read_content = io.read_all(&file_path).await?;
    assert_eq!(
        &read_content[..bytes_written],
        &content[..bytes_written],
        "Written content should match"
    );
    println!("✓ Partial write test passed");

    Ok(())
}

/// Test write with fsync using the platform backend.
/// **Validates: Requirements 11.5**
#[tokio::test]
async fn test_write_with_sync() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let io = create_platform_io();
    let backend_name = io.backend_name();

    println!("Testing write with sync using backend: {}", backend_name);

    // Create write operations with sync flag
    let sync_path = temp_dir.path().join("synced.txt");
    let sync_content = b"This content should be synced to disk";

    let write_ops = vec![WriteOp::with_sync(sync_path.clone(), sync_content.to_vec())];

    io.batch_write(&write_ops).await?;
    println!("✓ Write with sync completed");

    // Verify content
    let read_content = io.read_all(&sync_path).await?;
    assert_eq!(read_content, sync_content, "Synced content should match");
    println!("✓ Write with sync test passed");

    Ok(())
}

// ============================================================================
// Linux-Specific Tests (io_uring)
// **Validates: Requirements 11.1**
// ============================================================================

#[cfg(target_os = "linux")]
mod linux_tests {
    use super::*;
    use dx_forge::IoUringBackend;

    /// Test io_uring backend availability on Linux.
    /// **Validates: Requirements 11.1**
    #[tokio::test]
    async fn test_io_uring_availability() -> Result<()> {
        let is_available = IoUringBackend::is_available();
        println!("io_uring available: {}", is_available);

        if is_available {
            println!("✓ io_uring is available on this Linux system");

            // Verify kernel version
            if let Ok(output) = std::process::Command::new("uname").arg("-r").output() {
                if let Ok(version) = String::from_utf8(output.stdout) {
                    println!("  Kernel version: {}", version.trim());
                }
            }
        } else {
            println!("⚠ io_uring is not available (requires kernel 5.1+)");
        }

        Ok(())
    }

    /// Test io_uring backend directly if available.
    /// **Validates: Requirements 11.1, 11.5**
    #[tokio::test]
    async fn test_io_uring_direct_operations() -> Result<()> {
        if !IoUringBackend::is_available() {
            println!("Skipping io_uring direct test - not available");
            return Ok(());
        }

        let temp_dir = TempDir::new()?;
        let backend = IoUringBackend::new(256)?;

        println!("Testing io_uring backend directly");

        // Test write_all and read_all
        let file_path = temp_dir.path().join("io_uring_test.txt");
        let content = b"Testing io_uring direct operations";

        backend.write_all(&file_path, content).await?;
        let read_content = backend.read_all(&file_path).await?;

        assert_eq!(read_content, content, "io_uring read/write should work");
        println!("✓ io_uring direct read/write passed");

        // Test batch operations (io_uring's strength)
        let num_files = 20;
        let mut write_ops: Vec<WriteOp> = Vec::with_capacity(num_files);
        let mut paths: Vec<PathBuf> = Vec::with_capacity(num_files);

        for i in 0..num_files {
            let path = temp_dir.path().join(format!("io_uring_batch_{}.txt", i));
            let content = format!("io_uring batch content {}", i);
            write_ops.push(WriteOp::new(path.clone(), content.into_bytes()));
            paths.push(path);
        }

        backend.batch_write(&write_ops).await?;
        let contents = backend.batch_read(&paths).await?;

        assert_eq!(contents.len(), num_files, "Should read all batch files");
        println!("✓ io_uring batch operations passed ({} files)", num_files);

        assert_eq!(backend.backend_name(), "io_uring");
        println!("✓ io_uring backend name verified");

        Ok(())
    }
}

// ============================================================================
// macOS-Specific Tests (kqueue)
// **Validates: Requirements 11.2**
// ============================================================================

#[cfg(target_os = "macos")]
mod macos_tests {
    use super::*;
    use dx_forge::KqueueBackend;

    /// Test kqueue backend availability on macOS.
    /// **Validates: Requirements 11.2**
    #[tokio::test]
    async fn test_kqueue_availability() -> Result<()> {
        let is_available = KqueueBackend::is_available();
        println!("kqueue available: {}", is_available);

        // kqueue should always be available on macOS
        assert!(is_available, "kqueue should be available on macOS");
        println!("✓ kqueue is available on this macOS system");

        // Print macOS version
        if let Ok(output) = std::process::Command::new("sw_vers").arg("-productVersion").output() {
            if let Ok(version) = String::from_utf8(output.stdout) {
                println!("  macOS version: {}", version.trim());
            }
        }

        Ok(())
    }

    /// Test kqueue backend directly.
    /// **Validates: Requirements 11.2, 11.5**
    #[tokio::test]
    async fn test_kqueue_direct_operations() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let backend = KqueueBackend::new()?;

        println!("Testing kqueue backend directly");

        // Test write_all and read_all
        let file_path = temp_dir.path().join("kqueue_test.txt");
        let content = b"Testing kqueue direct operations";

        backend.write_all(&file_path, content).await?;
        let read_content = backend.read_all(&file_path).await?;

        assert_eq!(read_content, content, "kqueue read/write should work");
        println!("✓ kqueue direct read/write passed");

        // Test batch operations
        let num_files = 20;
        let mut write_ops: Vec<WriteOp> = Vec::with_capacity(num_files);
        let mut paths: Vec<PathBuf> = Vec::with_capacity(num_files);

        for i in 0..num_files {
            let path = temp_dir.path().join(format!("kqueue_batch_{}.txt", i));
            let content = format!("kqueue batch content {}", i);
            write_ops.push(WriteOp::new(path.clone(), content.into_bytes()));
            paths.push(path);
        }

        backend.batch_write(&write_ops).await?;
        let contents = backend.batch_read(&paths).await?;

        assert_eq!(contents.len(), num_files, "Should read all batch files");
        println!("✓ kqueue batch operations passed ({} files)", num_files);

        assert_eq!(backend.backend_name(), "kqueue");
        println!("✓ kqueue backend name verified");

        Ok(())
    }
}

// ============================================================================
// Windows-Specific Tests (IOCP)
// **Validates: Requirements 11.3**
// ============================================================================

#[cfg(target_os = "windows")]
mod windows_tests {
    use super::*;
    use dx_forge::IocpBackend;

    /// Test IOCP backend availability on Windows.
    /// **Validates: Requirements 11.3**
    #[tokio::test]
    async fn test_iocp_availability() -> Result<()> {
        let is_available = IocpBackend::is_available();
        println!("IOCP available: {}", is_available);

        // IOCP should always be available on Windows
        assert!(is_available, "IOCP should be available on Windows");
        println!("✓ IOCP is available on this Windows system");

        Ok(())
    }

    /// Test IOCP backend directly.
    /// **Validates: Requirements 11.3, 11.5**
    #[tokio::test]
    async fn test_iocp_direct_operations() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let backend = IocpBackend::new(num_cpus::get())?;

        println!("Testing IOCP backend directly");
        println!("  Thread count: {}", backend.thread_count());

        // Test write_all and read_all
        let file_path = temp_dir.path().join("iocp_test.txt");
        let content = b"Testing IOCP direct operations";

        backend.write_all(&file_path, content).await?;
        let read_content = backend.read_all(&file_path).await?;

        assert_eq!(read_content, content, "IOCP read/write should work");
        println!("✓ IOCP direct read/write passed");

        // Test batch operations
        let num_files = 20;
        let mut write_ops: Vec<WriteOp> = Vec::with_capacity(num_files);
        let mut paths: Vec<PathBuf> = Vec::with_capacity(num_files);

        for i in 0..num_files {
            let path = temp_dir.path().join(format!("iocp_batch_{}.txt", i));
            let content = format!("IOCP batch content {}", i);
            write_ops.push(WriteOp::new(path.clone(), content.into_bytes()));
            paths.push(path);
        }

        backend.batch_write(&write_ops).await?;
        let contents = backend.batch_read(&paths).await?;

        assert_eq!(contents.len(), num_files, "Should read all batch files");
        println!("✓ IOCP batch operations passed ({} files)", num_files);

        assert_eq!(backend.backend_name(), "iocp");
        println!("✓ IOCP backend name verified");

        Ok(())
    }
}

// ============================================================================
// Performance Comparison Tests
// **Validates: Requirements 11.5**
// ============================================================================

/// Compare sequential vs batch read performance.
/// **Validates: Requirements 11.5**
#[tokio::test]
async fn test_batch_vs_sequential_performance() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let io = create_platform_io();
    let backend_name = io.backend_name();

    println!("Testing batch vs sequential performance with backend: {}", backend_name);

    let num_files = 100;
    let file_size = 1000; // 1KB per file

    // Create test files
    let mut write_ops: Vec<WriteOp> = Vec::with_capacity(num_files);
    let mut paths: Vec<PathBuf> = Vec::with_capacity(num_files);

    for i in 0..num_files {
        let path = temp_dir.path().join(format!("perf_test_{}.bin", i));
        let content: Vec<u8> = (0..file_size).map(|j| ((i + j) % 256) as u8).collect();
        write_ops.push(WriteOp::new(path.clone(), content));
        paths.push(path);
    }

    io.batch_write(&write_ops).await?;

    // Measure sequential read time
    let seq_start = std::time::Instant::now();
    for path in &paths {
        let _ = io.read_all(path).await?;
    }
    let seq_elapsed = seq_start.elapsed();

    // Measure batch read time
    let batch_start = std::time::Instant::now();
    let _ = io.batch_read(&paths).await?;
    let batch_elapsed = batch_start.elapsed();

    println!("Sequential read of {} files: {:?}", num_files, seq_elapsed);
    println!("Batch read of {} files: {:?}", num_files, batch_elapsed);

    // Batch should generally be faster or at least not significantly slower
    // We don't assert this strictly as it depends on system load
    if batch_elapsed < seq_elapsed {
        let speedup = seq_elapsed.as_secs_f64() / batch_elapsed.as_secs_f64();
        println!("✓ Batch read was {:.2}x faster", speedup);
    } else {
        println!("⚠ Batch read was not faster (may be due to system load or small file count)");
    }

    Ok(())
}

/// Test concurrent I/O operations using the platform backend.
/// **Validates: Requirements 11.5**
#[tokio::test]
async fn test_concurrent_io_operations() -> Result<()> {
    use std::sync::Arc;
    use tokio::sync::Barrier;

    let temp_dir = TempDir::new()?;
    let io = Arc::new(create_platform_io());
    let backend_name = io.backend_name();

    println!("Testing concurrent I/O operations with backend: {}", backend_name);

    let num_tasks = 50;
    let barrier = Arc::new(Barrier::new(num_tasks));

    // Create initial files
    for i in 0..num_tasks {
        let path = temp_dir.path().join(format!("concurrent_{}.txt", i));
        io.write_all(&path, format!("Initial content {}", i).as_bytes()).await?;
    }

    // Spawn concurrent read/write tasks
    let mut handles = Vec::with_capacity(num_tasks);
    for i in 0..num_tasks {
        let io = Arc::clone(&io);
        let path = temp_dir.path().join(format!("concurrent_{}.txt", i));
        let barrier = Arc::clone(&barrier);

        let handle = tokio::spawn(async move {
            // Wait for all tasks to be ready
            barrier.wait().await;

            // Perform read
            let content = io.read_all(&path).await?;
            assert!(!content.is_empty(), "File {} should have content", i);

            // Perform write
            let new_content = format!("Updated content {} at {:?}", i, std::time::Instant::now());
            io.write_all(&path, new_content.as_bytes()).await?;

            // Verify write
            let final_content = io.read_all(&path).await?;
            assert!(!final_content.is_empty(), "File {} should have updated content", i);

            Ok::<_, anyhow::Error>(())
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await??;
    }

    println!("✓ {} concurrent I/O operations completed successfully", num_tasks);

    Ok(())
}
