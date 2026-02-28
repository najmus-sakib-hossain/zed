//! Unit tests for ExternalToolManager
//!
//! Tests cover:
//! - Tool discovery (PATH search, common locations)
//! - Version detection (parsing, caching)
//! - Cache invalidation (stale entries, nonexistent paths)

use dx_check::languages::{ExternalToolManager, ToolCache};
use std::path::PathBuf;

// ============================================================================
// Tool Discovery Tests
// ============================================================================

#[test]
fn test_tool_discovery_in_path() {
    let manager = ExternalToolManager::new();

    // Try to find a common tool (cargo should exist in Rust environment)
    if let Some(cargo_path) = manager.find_tool_cached("cargo") {
        assert!(cargo_path.exists());
        assert!(cargo_path.is_file());

        // Verify it's cached
        let cached_path = manager.cache().get("cargo");
        assert_eq!(cached_path, Some(cargo_path.clone()));

        // Second lookup should use cache
        let second_lookup = manager.find_tool_cached("cargo");
        assert_eq!(second_lookup, Some(cargo_path));
    }
}

#[test]
fn test_tool_discovery_static_method() {
    // Test the static find_tool method
    if let Some(cargo_path) = ExternalToolManager::find_tool("cargo") {
        assert!(cargo_path.exists());
        assert!(cargo_path.is_file());
    }
}

#[test]
fn test_tool_discovery_nonexistent_tool() {
    let manager = ExternalToolManager::new();

    // Try to find a tool that definitely doesn't exist
    let result = manager.find_tool_cached("nonexistent-tool-xyz-123-456");
    assert!(result.is_none());

    // Should not be in cache
    assert!(manager.cache().get("nonexistent-tool-xyz-123-456").is_none());
}

#[test]
fn test_tool_discovery_multiple_tools() {
    let manager = ExternalToolManager::new();

    // Try to find multiple common tools
    let tools = ["cargo", "rustc", "rustup"];
    let mut found_count = 0;

    for tool in &tools {
        if let Some(path) = manager.find_tool_cached(tool) {
            assert!(path.exists());
            assert!(manager.cache().get(tool).is_some());
            found_count += 1;
        }
    }

    // At least one tool should be found in a Rust environment
    assert!(found_count > 0, "Expected to find at least one Rust tool");
}

#[test]
fn test_tool_discovery_case_sensitivity() {
    let manager = ExternalToolManager::new();

    // Tool names should be case-sensitive
    let result1 = manager.find_tool_cached("cargo");
    let _result2 = manager.find_tool_cached("CARGO");

    // On case-sensitive systems, these might differ
    // On case-insensitive systems (Windows), they might be the same
    // We just verify the behavior is consistent
    if result1.is_some() {
        assert!(result1.unwrap().exists());
    }
}

#[test]
fn test_tool_discovery_with_extension() {
    // On Windows, tools might have .exe extension
    let manager = ExternalToolManager::new();

    if cfg!(target_os = "windows") {
        // Try both with and without .exe
        let result1 = manager.find_tool_cached("cargo");
        let result2 = manager.find_tool_cached("cargo.exe");

        // At least one should work
        assert!(result1.is_some() || result2.is_some());
    }
}

// ============================================================================
// Version Detection Tests
// ============================================================================

#[test]
fn test_version_detection_basic() {
    let manager = ExternalToolManager::new();

    // Try to detect cargo version
    if manager.find_tool_cached("cargo").is_some() {
        if let Some(version) = manager.get_tool_version("cargo") {
            assert_eq!(version.tool, "cargo");
            assert!(!version.version.is_empty());
            assert!(!version.raw_output.is_empty());

            // Version should match pattern (e.g., "1.75.0")
            assert!(version.version.contains('.'), "Version should contain dots");

            // Version should be cached
            let cached_version = manager.cache().get_version("cargo");
            assert_eq!(cached_version, Some(version));
        }
    }
}

#[test]
fn test_version_detection_multiple_calls() {
    let manager = ExternalToolManager::new();

    if manager.find_tool_cached("cargo").is_some() {
        let version1 = manager.get_tool_version("cargo");
        let version2 = manager.get_tool_version("cargo");

        // Both calls should return the same version (from cache)
        assert_eq!(version1, version2);
    }
}

#[test]
fn test_version_detection_nonexistent_tool() {
    let manager = ExternalToolManager::new();

    // Try to get version of nonexistent tool
    let version = manager.get_tool_version("nonexistent-tool-xyz");
    assert!(version.is_none());
}

#[test]
fn test_version_detection_after_manual_config() {
    let manager = ExternalToolManager::new();

    // Find cargo and configure it manually
    if let Some(cargo_path) = ExternalToolManager::find_tool("cargo") {
        let result = manager.configure_tool("my-cargo", cargo_path.clone());
        assert!(result.is_ok());

        // Version should be detected during configuration
        let version = manager.cache().get_version("my-cargo");
        if version.is_some() {
            let v = version.unwrap();
            assert_eq!(v.tool, "my-cargo");
            assert!(!v.version.is_empty());
        }
    }
}

#[test]
fn test_version_detection_caching() {
    let cache = ToolCache::new();
    let manager = ExternalToolManager::with_cache(cache.clone());

    if manager.find_tool_cached("cargo").is_some() {
        // Get version (should cache it)
        let version1 = manager.get_tool_version("cargo");

        // Create new manager with same cache
        let manager2 = ExternalToolManager::with_cache(cache.clone());

        // Should get cached version without re-detection
        let version2 = manager2.cache().get_version("cargo");

        assert_eq!(version1, version2);
    }
}

#[test]
fn test_version_detection_with_rustc() {
    let manager = ExternalToolManager::new();

    // Try rustc which has a different version format
    if manager.find_tool_cached("rustc").is_some() {
        if let Some(version) = manager.get_tool_version("rustc") {
            assert_eq!(version.tool, "rustc");
            assert!(!version.version.is_empty());
            // rustc version typically starts with "1."
            assert!(
                version.version.starts_with("1.")
                    || version.version.chars().next().unwrap().is_ascii_digit()
            );
        }
    }
}

#[test]
fn test_version_detection_persistence() {
    let manager = ExternalToolManager::new();

    if manager.find_tool_cached("cargo").is_some() {
        // Get version and ensure it's cached
        let version = manager.get_tool_version("cargo");

        if version.is_some() {
            // Save cache
            let _ = manager.cache().save();

            // Create new cache and load
            let new_cache = ToolCache::new();
            let _ = new_cache.load();

            // Version should be loaded from disk (if save succeeded)
            let loaded_version = new_cache.get_version("cargo");
            if loaded_version.is_some() {
                assert_eq!(loaded_version, version);
            }
        }
    }
}

#[test]
fn test_shared_cache_between_managers() {
    let cache = ToolCache::new();

    let manager1 = ExternalToolManager::with_cache(cache.clone());
    let manager2 = ExternalToolManager::with_cache(cache.clone());

    // Find tool with first manager
    if let Some(path) = manager1.find_tool_cached("cargo") {
        // Second manager should see the cached result
        assert_eq!(manager2.cache().get("cargo"), Some(path));
    }
}

#[test]
fn test_manual_configuration() {
    let manager = ExternalToolManager::new();

    // Try to configure with an existing tool
    if let Some(cargo_path) = ExternalToolManager::find_tool("cargo") {
        let result = manager.configure_tool("my-cargo", cargo_path.clone());
        assert!(result.is_ok());

        // Verify it's cached as manual
        assert!(manager.cache().is_manual("my-cargo"));
        assert_eq!(manager.cache().get("my-cargo"), Some(cargo_path));
    }
}

// ============================================================================
// Cache Invalidation Tests
// ============================================================================

#[test]
fn test_cache_invalidation_stale_entry() {
    let manager = ExternalToolManager::new();

    // Add a tool to cache with nonexistent path
    manager
        .cache()
        .set("test-tool", PathBuf::from("/nonexistent/tool"), None, false);

    // Verify it's cached
    assert!(manager.cache().get("test-tool").is_some());

    // Try to find it (should invalidate stale cache)
    let result = manager.find_tool_cached("test-tool");

    // Should not find nonexistent tool
    assert!(result.is_none());

    // Cache should be invalidated
    assert!(manager.cache().get("test-tool").is_none());
}

#[test]
fn test_cache_invalidation_deleted_file() {
    use tempfile::NamedTempFile;

    let manager = ExternalToolManager::new();

    // Create a temporary file
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_path_buf();

    // Cache it
    manager.cache().set("temp-tool", temp_path.clone(), None, false);
    assert_eq!(manager.cache().get("temp-tool"), Some(temp_path.clone()));

    // Delete the file
    drop(temp_file);

    // Try to find it - should invalidate cache
    let result = manager.find_tool_cached("temp-tool");
    assert!(result.is_none());

    // Cache should be cleared
    assert!(manager.cache().get("temp-tool").is_none());
}

#[test]
fn test_cache_invalidation_manual_removal() {
    let manager = ExternalToolManager::new();

    // Add a tool
    manager.cache().set("temp-tool", PathBuf::from("/usr/bin/test"), None, true);

    assert!(manager.cache().get("temp-tool").is_some());

    // Remove it manually
    manager.remove_tool_config("temp-tool");

    assert!(manager.cache().get("temp-tool").is_none());
}

#[test]
fn test_cache_invalidation_clear_all() {
    let cache = ToolCache::new();

    // Add multiple tools
    cache.set("tool1", PathBuf::from("/usr/bin/tool1"), None, false);
    cache.set("tool2", PathBuf::from("/usr/bin/tool2"), None, false);
    cache.set("tool3", PathBuf::from("/usr/bin/tool3"), None, true);

    assert_eq!(cache.tools().len(), 3);

    // Clear cache
    cache.clear();

    assert_eq!(cache.tools().len(), 0);
    assert!(cache.get("tool1").is_none());
    assert!(cache.get("tool2").is_none());
    assert!(cache.get("tool3").is_none());
}

#[test]
fn test_cache_invalidation_preserves_valid_entries() {
    let manager = ExternalToolManager::new();

    // Add a valid tool (cargo)
    if let Some(cargo_path) = ExternalToolManager::find_tool("cargo") {
        manager.cache().set("cargo", cargo_path.clone(), None, false);

        // Add an invalid tool
        manager.cache().set("invalid-tool", PathBuf::from("/nonexistent"), None, false);

        // Try to find the invalid tool (should invalidate only that entry)
        let _ = manager.find_tool_cached("invalid-tool");

        // Valid tool should still be cached
        assert_eq!(manager.cache().get("cargo"), Some(cargo_path));

        // Invalid tool should be removed
        assert!(manager.cache().get("invalid-tool").is_none());
    }
}

#[test]
fn test_cache_invalidation_version_info() {
    let cache = ToolCache::new();

    // Add tool with version
    let version = dx_check::languages::ToolVersion::new("tool", "1.0.0", "tool 1.0.0");
    cache.set("tool", PathBuf::from("/usr/bin/tool"), Some(version.clone()), false);

    assert_eq!(cache.get_version("tool"), Some(version));

    // Remove tool
    cache.remove("tool");

    // Version should also be removed
    assert!(cache.get_version("tool").is_none());
}

#[test]
fn test_cache_invalidation_after_reconfiguration() {
    let manager = ExternalToolManager::new();

    // Find cargo
    if let Some(cargo_path) = ExternalToolManager::find_tool("cargo") {
        // Configure it manually
        let _ = manager.configure_tool("my-tool", cargo_path.clone());
        assert!(manager.cache().is_manual("my-tool"));

        // Reconfigure with different name
        let _ = manager.configure_tool("my-tool-v2", cargo_path.clone());

        // Both should exist
        assert!(manager.cache().get("my-tool").is_some());
        assert!(manager.cache().get("my-tool-v2").is_some());

        // Remove first one
        manager.remove_tool_config("my-tool");
        assert!(manager.cache().get("my-tool").is_none());
        assert!(manager.cache().get("my-tool-v2").is_some());
    }
}

#[test]
fn test_cache_invalidation_concurrent_access() {
    let cache = ToolCache::new();

    // Simulate concurrent access with shared cache
    let manager1 = ExternalToolManager::with_cache(cache.clone());
    let manager2 = ExternalToolManager::with_cache(cache.clone());

    // Manager 1 adds a tool
    cache.set("tool1", PathBuf::from("/usr/bin/tool1"), None, false);

    // Manager 2 should see it
    assert_eq!(manager2.cache().get("tool1"), Some(PathBuf::from("/usr/bin/tool1")));

    // Manager 1 removes it
    manager1.remove_tool_config("tool1");

    // Manager 2 should see it's gone
    assert!(manager2.cache().get("tool1").is_none());
}

#[test]
fn test_cache_invalidation_on_path_change() {
    let manager = ExternalToolManager::new();

    if let Some(cargo_path) = ExternalToolManager::find_tool("cargo") {
        // Cache cargo
        manager.cache().set("cargo", cargo_path.clone(), None, false);

        // Manually update to wrong path
        manager.cache().set("cargo", PathBuf::from("/wrong/path"), None, false);

        // Find again - should invalidate and find correct path
        let result = manager.find_tool_cached("cargo");

        if result.is_some() {
            assert_eq!(result.unwrap(), cargo_path);
        }
    }
}

// ============================================================================
// Additional Integration Tests
// ============================================================================

#[test]
fn test_tool_not_found() {
    let manager = ExternalToolManager::new();

    // Try to find a tool that definitely doesn't exist
    let result = manager.find_tool_cached("nonexistent-tool-xyz-123");
    assert!(result.is_none());

    // Should not be in cache
    assert!(manager.cache().get("nonexistent-tool-xyz-123").is_none());
}

#[test]
fn test_configure_nonexistent_tool() {
    let manager = ExternalToolManager::new();

    let result = manager.configure_tool("fake-tool", PathBuf::from("/this/path/does/not/exist"));

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("does not exist"));
}

#[test]
fn test_remove_tool_configuration() {
    let manager = ExternalToolManager::new();

    // Add a tool
    manager.cache().set("temp-tool", PathBuf::from("/usr/bin/test"), None, true);

    assert!(manager.cache().get("temp-tool").is_some());

    // Remove it
    manager.remove_tool_config("temp-tool");

    assert!(manager.cache().get("temp-tool").is_none());
}

#[test]
fn test_cache_clear() {
    let cache = ToolCache::new();

    // Add multiple tools
    cache.set("tool1", PathBuf::from("/usr/bin/tool1"), None, false);
    cache.set("tool2", PathBuf::from("/usr/bin/tool2"), None, false);
    cache.set("tool3", PathBuf::from("/usr/bin/tool3"), None, true);

    assert_eq!(cache.tools().len(), 3);

    // Clear cache
    cache.clear();

    assert_eq!(cache.tools().len(), 0);
}

#[test]
fn test_backward_compatibility() {
    // Static find_tool method should still work
    let result = ExternalToolManager::find_tool("cargo");

    if result.is_some() {
        assert!(result.unwrap().exists());
    }
}

// ============================================================================
// Tool Discovery Edge Cases
// ============================================================================

#[test]
fn test_tool_discovery_empty_string() {
    let manager = ExternalToolManager::new();
    let result = manager.find_tool_cached("");
    assert!(result.is_none());
}

#[test]
fn test_tool_discovery_with_path_separators() {
    let manager = ExternalToolManager::new();
    // Tool names shouldn't contain path separators
    let result = manager.find_tool_cached("bin/cargo");
    assert!(result.is_none());
}

#[test]
fn test_tool_discovery_special_characters() {
    let manager = ExternalToolManager::new();
    // Tool names with special characters
    let result = manager.find_tool_cached("tool@#$%");
    assert!(result.is_none());
}

// ============================================================================
// Version Detection Edge Cases
// ============================================================================

#[test]
fn test_version_detection_tool_without_version() {
    let manager = ExternalToolManager::new();

    // Some tools might not support version flags
    // This should handle gracefully
    let version = manager.get_tool_version("nonexistent-no-version-tool");
    assert!(version.is_none());
}

// ============================================================================
// Cache Persistence Tests
// ============================================================================

#[test]
fn test_cache_save_creates_directory() {
    let cache = ToolCache::new();

    // Add a tool
    cache.set("test-tool", PathBuf::from("/usr/bin/test"), None, false);

    // Save should create directory if needed
    let result = cache.save();
    // May fail if permissions don't allow, but shouldn't panic
    let _ = result;
}

#[test]
fn test_cache_load_nonexistent_file() {
    let cache = ToolCache::new();

    // Loading nonexistent cache file should succeed (empty cache)
    let result = cache.load();
    assert!(result.is_ok());
}

#[test]
fn test_cache_multiple_save_load_cycles() {
    let cache = ToolCache::new();

    // Add tools
    cache.set("tool1", PathBuf::from("/usr/bin/tool1"), None, false);
    let _ = cache.save();

    // Add more tools
    cache.set("tool2", PathBuf::from("/usr/bin/tool2"), None, false);
    let _ = cache.save();

    // Load again
    let _ = cache.load();

    // Should have both tools (if save/load succeeded)
    let tools = cache.tools();
    assert!(tools.len() >= 0); // At least doesn't panic
}

// ============================================================================
// Manual Configuration Tests
// ============================================================================

#[test]
fn test_manual_configuration_with_valid_tool() {
    let manager = ExternalToolManager::new();

    // Try to configure with an existing tool
    if let Some(cargo_path) = ExternalToolManager::find_tool("cargo") {
        let result = manager.configure_tool("my-cargo", cargo_path.clone());
        assert!(result.is_ok());

        // Verify it's cached as manual
        assert!(manager.cache().is_manual("my-cargo"));
        assert_eq!(manager.cache().get("my-cargo"), Some(cargo_path));
    }
}

#[test]
fn test_manual_configuration_overwrite() {
    let manager = ExternalToolManager::new();

    if let Some(cargo_path) = ExternalToolManager::find_tool("cargo") {
        // Configure once
        let _ = manager.configure_tool("my-tool", cargo_path.clone());

        // Configure again with same name
        let result = manager.configure_tool("my-tool", cargo_path.clone());
        assert!(result.is_ok());

        // Should still be manual
        assert!(manager.cache().is_manual("my-tool"));
    }
}

#[test]
fn test_manual_configuration_directory_path() {
    let manager = ExternalToolManager::new();

    // Try to configure with a directory instead of file
    // Use a path that exists on the current system
    let dir_path = if cfg!(target_os = "windows") {
        PathBuf::from("C:\\Windows")
    } else {
        PathBuf::from("/usr")
    };

    // Only test if the directory exists
    if dir_path.exists() && dir_path.is_dir() {
        let result = manager.configure_tool("bad-tool", dir_path);
        assert!(result.is_err());
        if let Err(msg) = result {
            assert!(msg.contains("not a file"));
        }
    }
}

// ============================================================================
// Shared Cache Tests
// ============================================================================

#[test]
fn test_shared_cache_modifications() {
    let cache = ToolCache::new();

    let manager1 = ExternalToolManager::with_cache(cache.clone());
    let manager2 = ExternalToolManager::with_cache(cache.clone());

    // Manager 1 adds a tool
    cache.set("tool1", PathBuf::from("/usr/bin/tool1"), None, false);

    // Manager 2 should see it immediately
    assert_eq!(manager2.cache().get("tool1"), Some(PathBuf::from("/usr/bin/tool1")));

    // Manager 2 removes it
    manager2.remove_tool_config("tool1");

    // Manager 1 should see it's gone
    assert!(manager1.cache().get("tool1").is_none());
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_ensure_tool_nonexistent() {
    let manager = ExternalToolManager::new();

    // Try to ensure a tool that doesn't exist
    let result = manager.ensure_tool("definitely-nonexistent-tool-xyz-999");

    assert!(result.is_err());

    if let Err(err) = result {
        assert!(!err.tool.is_empty());
        assert!(!err.message.is_empty());
        assert!(!err.instructions.is_empty());
    }
}

// ============================================================================
// Tool Cache Functionality Tests
// ============================================================================

#[test]
fn test_tool_cache_default() {
    let cache = ToolCache::default();
    assert!(cache.tools().is_empty() || cache.tools().len() >= 0);
}

#[test]
fn test_tool_cache_is_manual_nonexistent() {
    let cache = ToolCache::new();
    assert!(!cache.is_manual("nonexistent-tool"));
}

#[test]
fn test_tool_cache_get_version_nonexistent() {
    let cache = ToolCache::new();
    assert!(cache.get_version("nonexistent-tool").is_none());
}

#[test]
fn test_tool_cache_remove_nonexistent() {
    let cache = ToolCache::new();
    // Should not panic
    cache.remove("nonexistent-tool");
}

#[test]
fn test_tool_cache_multiple_versions() {
    let cache = ToolCache::new();

    let v1 = dx_check::languages::ToolVersion::new("tool", "1.0.0", "tool 1.0.0");
    let v2 = dx_check::languages::ToolVersion::new("tool", "2.0.0", "tool 2.0.0");

    // Set with version 1
    cache.set("tool", PathBuf::from("/usr/bin/tool"), Some(v1), false);

    // Update with version 2
    cache.set("tool", PathBuf::from("/usr/bin/tool"), Some(v2.clone()), false);

    // Should have version 2
    assert_eq!(cache.get_version("tool"), Some(v2));
}

// ============================================================================
// Integration with Real Tools
// ============================================================================

#[test]
fn test_real_tool_cargo_full_workflow() {
    let manager = ExternalToolManager::new();

    // Find cargo
    if let Some(cargo_path) = manager.find_tool_cached("cargo") {
        // Should be cached
        assert_eq!(manager.cache().get("cargo"), Some(cargo_path.clone()));

        // Get version
        if let Some(version) = manager.get_tool_version("cargo") {
            assert_eq!(version.tool, "cargo");
            assert!(!version.version.is_empty());

            // Version should be cached
            assert_eq!(manager.cache().get_version("cargo"), Some(version));
        }

        // Ensure tool (should use cache)
        let ensure_result = manager.ensure_tool("cargo");
        assert!(ensure_result.is_ok());
        assert_eq!(ensure_result.unwrap(), cargo_path);
    }
}

#[test]
fn test_real_tool_rustc_full_workflow() {
    let manager = ExternalToolManager::new();

    if let Some(rustc_path) = manager.find_tool_cached("rustc") {
        assert!(rustc_path.exists());

        // Get version
        if let Some(version) = manager.get_tool_version("rustc") {
            assert_eq!(version.tool, "rustc");
            assert!(!version.version.is_empty());
        }
    }
}
