//! Example demonstrating ExternalToolManager usage

use dx_check::languages::{ExternalToolManager, ToolCache};
use std::path::PathBuf;

fn main() {
    println!("=== ExternalToolManager Example ===\n");

    // Create a new tool manager
    let manager = ExternalToolManager::new();

    // 1. Tool Discovery with Caching
    println!("1. Tool Discovery:");
    if let Some(cargo_path) = manager.find_tool_cached("cargo") {
        println!("   Found cargo at: {}", cargo_path.display());

        // Second lookup uses cache
        if let Some(cached) = manager.cache().get("cargo") {
            println!("   Cached path: {}", cached.display());
        }
    }

    // 2. Version Detection
    println!("\n2. Version Detection:");
    if let Some(version) = manager.get_tool_version("cargo") {
        println!("   Tool: {}", version.tool);
        println!("   Version: {}", version.version);
        println!("   Raw output: {}", version.raw_output.lines().next().unwrap_or(""));
    }

    // 3. Auto-Installation (ensure_tool)
    println!("\n3. Auto-Installation:");
    match manager.ensure_tool("rustfmt") {
        Ok(path) => {
            println!("   rustfmt available at: {}", path.display());
            println!("   (Found in PATH or auto-installed)");
        }
        Err(e) => {
            println!("   Failed to ensure rustfmt: {}", e.message);
            println!("   Installation instructions:");
            println!("{}", e.instructions);
        }
    }

    // 4. Manual Configuration
    println!("\n4. Manual Configuration:");
    if let Some(rustc_path) = ExternalToolManager::find_tool("rustc") {
        match manager.configure_tool("my-rustc", rustc_path.clone()) {
            Ok(()) => {
                println!("   Configured 'my-rustc' at: {}", rustc_path.display());
                println!("   Is manual: {}", manager.cache().is_manual("my-rustc"));
            }
            Err(e) => println!("   Error: {}", e),
        }
    }

    // 5. Shared Cache
    println!("\n5. Shared Cache:");
    let shared_cache = ToolCache::new();
    let manager1 = ExternalToolManager::with_cache(shared_cache.clone());
    let manager2 = ExternalToolManager::with_cache(shared_cache.clone());

    if manager1.find_tool_cached("rustfmt").is_some() {
        println!("   Manager 1 found rustfmt");
        if manager2.cache().get("rustfmt").is_some() {
            println!("   Manager 2 sees cached rustfmt");
        }
    }

    // 6. Cache Persistence
    println!("\n6. Cache Persistence:");
    println!("   Cache saved to: .dx/cache/tools/tools.json");
    if let Err(e) = manager.cache().save() {
        println!("   Error saving cache: {}", e);
    } else {
        println!("   Cache saved successfully");
    }

    // 7. Cache Management
    println!("\n7. Cache Management:");
    println!("   Cached tools: {:?}", manager.cache().tools());

    // Clear specific tool
    manager.remove_tool_config("my-rustc");
    println!("   After removing 'my-rustc': {:?}", manager.cache().tools());

    println!("\n=== Example Complete ===");
}
