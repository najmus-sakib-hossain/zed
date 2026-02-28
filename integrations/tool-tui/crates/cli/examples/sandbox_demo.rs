//! Sandbox demo - shows how to use the sandbox API

use dx::sandbox::{NetworkMode, ResourceLimits, SandboxBackendType, SandboxConfig, SandboxManager};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("DX Sandbox Demo\n");

    // Create sandbox manager
    let manager = SandboxManager::new();

    // Configure sandbox with resource limits
    let config = SandboxConfig {
        limits: ResourceLimits {
            memory_bytes: Some(256 * 1024 * 1024), // 256 MB
            cpu_shares: Some(512),                 // 50% CPU
            disk_bytes: Some(512 * 1024 * 1024),   // 512 MB
            max_pids: Some(50),
            max_files: Some(512),
        },
        network: NetworkMode::None,
        network_enabled: false,
        ..Default::default()
    };

    // Create sandbox
    println!("Creating sandbox...");
    let sandbox_id = manager.create("demo".to_string(), SandboxBackendType::Auto, config).await?;

    println!("Sandbox created: {}\n", sandbox_id);

    // Execute command
    println!("Executing command in sandbox...");
    let result = manager
        .execute(&sandbox_id, &["echo".to_string(), "Hello from sandbox!".to_string()])
        .await?;

    println!("Exit code: {}", result.exit_code);
    println!("Output: {}", result.stdout);
    println!("Duration: {}ms\n", result.duration_ms);

    // Cleanup
    println!("Destroying sandbox...");
    manager.destroy(&sandbox_id).await?;

    println!("Done!");

    Ok(())
}
