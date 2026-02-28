//! Property-based tests for configuration layering
//!
//! **Property 9: Configuration Layering**
//! *For any* configuration key, the value SHALL be determined by the highest-priority
//! source (env var > project config > global config > default).
//!
//! **Validates: Requirements 11.1, 11.3**

use proptest::prelude::*;
use std::sync::Mutex;
use tempfile::TempDir;

use dx_py_compat::{Config, ConfigFile, DEFAULT_INDEX_URL};

// Mutex to ensure env var tests don't interfere with each other
static ENV_MUTEX: Mutex<()> = Mutex::new(());

/// Strategy for generating valid index URLs
fn index_url_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex("https://[a-z]+\\.(pypi|example)\\.org/simple/")
        .unwrap()
        .prop_filter("non-empty", |s| !s.is_empty())
}

/// Strategy for generating max concurrent downloads (reasonable range)
fn max_concurrent_strategy() -> impl Strategy<Value = usize> {
    1usize..32usize
}

/// Strategy for generating retry counts
fn retry_count_strategy() -> impl Strategy<Value = u32> {
    1u32..10u32
}

/// Strategy for generating download timeouts
fn timeout_strategy() -> impl Strategy<Value = u64> {
    30u64..600u64
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// **Feature: dx-py-hardening, Property 9: Configuration Layering**
    ///
    /// Test that project config overrides global config.
    /// For any value set in project config, it SHALL override global config.
    #[test]
    fn prop_project_config_overrides_global(
        global_max in max_concurrent_strategy(),
        project_max in max_concurrent_strategy(),
    ) {
        let mut config = Config::default();

        // Simulate global config
        let global_config = ConfigFile {
            max_concurrent_downloads: Some(global_max),
            ..Default::default()
        };
        config.merge(&global_config);

        // Simulate project config (applied after global)
        let project_config = ConfigFile {
            max_concurrent_downloads: Some(project_max),
            ..Default::default()
        };
        config.merge(&project_config);

        // Project config should win
        prop_assert_eq!(config.max_concurrent_downloads, project_max);
    }

    /// **Feature: dx-py-hardening, Property 9: Configuration Layering**
    ///
    /// Test that defaults are used when no config is provided.
    /// For any config key not set in any source, the default value SHALL be used.
    #[test]
    fn prop_defaults_used_when_no_override(_seed in any::<u64>()) {
        let config = Config::default();

        // All defaults should be set
        prop_assert_eq!(config.index_url, DEFAULT_INDEX_URL);
        prop_assert_eq!(config.max_concurrent_downloads, 8);
        prop_assert_eq!(config.download_timeout, 300);
        prop_assert_eq!(config.retry_count, 3);
        prop_assert!(config.python_downloads);
        prop_assert!(config.use_hard_links);
        prop_assert!(!config.verbose);
        prop_assert!(!config.offline);
    }

    /// **Feature: dx-py-hardening, Property 9: Configuration Layering**
    ///
    /// Test that partial configs only override specified values.
    /// For any partial config, only the specified keys SHALL be overridden.
    #[test]
    fn prop_partial_config_preserves_unset_values(
        new_retry_count in retry_count_strategy(),
        new_timeout in timeout_strategy(),
    ) {
        let mut config = Config::default();
        let original_index_url = config.index_url.clone();
        let original_max_concurrent = config.max_concurrent_downloads;

        // Apply partial config that only sets retry_count and timeout
        let partial_config = ConfigFile {
            retry_count: Some(new_retry_count),
            download_timeout: Some(new_timeout),
            ..Default::default()
        };
        config.merge(&partial_config);

        // Changed values
        prop_assert_eq!(config.retry_count, new_retry_count);
        prop_assert_eq!(config.download_timeout, new_timeout);

        // Unchanged values
        prop_assert_eq!(config.index_url, original_index_url);
        prop_assert_eq!(config.max_concurrent_downloads, original_max_concurrent);
    }

    /// **Feature: dx-py-hardening, Property 9: Configuration Layering**
    ///
    /// Test full layering: global < project (merge order)
    /// For any configuration with multiple layers, later merges SHALL override earlier ones.
    #[test]
    fn prop_merge_order_determines_priority(
        global_val in max_concurrent_strategy(),
        project_val in max_concurrent_strategy(),
    ) {
        let default_val = 8usize;

        // Start with defaults
        let mut config = Config::default();
        prop_assert_eq!(config.max_concurrent_downloads, default_val);

        // Apply global config
        let global_config = ConfigFile {
            max_concurrent_downloads: Some(global_val),
            ..Default::default()
        };
        config.merge(&global_config);
        prop_assert_eq!(config.max_concurrent_downloads, global_val);

        // Apply project config (should override global)
        let project_config = ConfigFile {
            max_concurrent_downloads: Some(project_val),
            ..Default::default()
        };
        config.merge(&project_config);
        prop_assert_eq!(config.max_concurrent_downloads, project_val);
    }

    /// **Feature: dx-py-hardening, Property 9: Configuration Layering**
    ///
    /// Test that index URL layering works correctly.
    /// For any index_url values, later merges SHALL override earlier ones.
    #[test]
    fn prop_index_url_layering(
        global_url in index_url_strategy(),
        project_url in index_url_strategy(),
    ) {
        let mut config = Config::default();

        // Apply global config
        let global_config = ConfigFile {
            index_url: Some(global_url.clone()),
            ..Default::default()
        };
        config.merge(&global_config);
        prop_assert_eq!(&config.index_url, &global_url);

        // Apply project config (should override)
        let project_config = ConfigFile {
            index_url: Some(project_url.clone()),
            ..Default::default()
        };
        config.merge(&project_config);
        prop_assert_eq!(&config.index_url, &project_url);
    }
}

/// **Feature: dx-py-hardening, Property 9: Configuration Layering**
///
/// Test that environment variables override all other config sources.
#[test]
fn test_env_var_overrides_all_sources() {
    use std::env;
    let _guard = ENV_MUTEX.lock().unwrap();

    // Save original env var
    let orig = env::var("DX_PY_INDEX_URL").ok();

    // Set env var
    env::set_var("DX_PY_INDEX_URL", "https://env.pypi.org/simple/");

    // Create a config and apply env vars
    let mut config = Config::default();

    // First merge a "project config" with different value
    let project_config = ConfigFile {
        index_url: Some("https://project.pypi.org/simple/".to_string()),
        ..Default::default()
    };
    config.merge(&project_config);

    // Then apply env vars (should override)
    config.apply_env_vars();

    // Env var should win
    assert_eq!(config.index_url, "https://env.pypi.org/simple/");

    // Restore original
    match orig {
        Some(v) => env::set_var("DX_PY_INDEX_URL", v),
        None => env::remove_var("DX_PY_INDEX_URL"),
    }
}

/// **Feature: dx-py-hardening, Property 9: Configuration Layering**
///
/// Test that config can be loaded from a project directory with pyproject.toml
#[test]
fn test_load_from_project_dir() {
    use std::env;
    let _guard = ENV_MUTEX.lock().unwrap();

    let temp_dir = TempDir::new().unwrap();
    let pyproject_path = temp_dir.path().join("pyproject.toml");

    // Write a pyproject.toml with dx-py config
    std::fs::write(
        &pyproject_path,
        r#"
[project]
name = "test-project"

[tool.dx-py]
index_url = "https://custom.pypi.org/simple/"
max_concurrent_downloads = 16
verbose = true
"#,
    )
    .unwrap();

    // Clear any env vars that might interfere
    let orig_index = env::var("DX_PY_INDEX_URL").ok();
    let orig_max = env::var("DX_PY_MAX_CONCURRENT_DOWNLOADS").ok();
    let orig_verbose = env::var("DX_PY_VERBOSE").ok();
    env::remove_var("DX_PY_INDEX_URL");
    env::remove_var("DX_PY_MAX_CONCURRENT_DOWNLOADS");
    env::remove_var("DX_PY_VERBOSE");

    let config = Config::load_from_dir(temp_dir.path()).unwrap();

    assert_eq!(config.index_url, "https://custom.pypi.org/simple/");
    assert_eq!(config.max_concurrent_downloads, 16);
    assert!(config.verbose);

    // Restore env vars
    if let Some(v) = orig_index {
        env::set_var("DX_PY_INDEX_URL", v);
    }
    if let Some(v) = orig_max {
        env::set_var("DX_PY_MAX_CONCURRENT_DOWNLOADS", v);
    }
    if let Some(v) = orig_verbose {
        env::set_var("DX_PY_VERBOSE", v);
    }
}

/// Test that missing project dir doesn't cause errors
#[test]
fn test_load_from_empty_dir() {
    use std::env;
    let _guard = ENV_MUTEX.lock().unwrap();

    let temp_dir = TempDir::new().unwrap();

    // Clear env vars
    let orig = env::var("DX_PY_INDEX_URL").ok();
    env::remove_var("DX_PY_INDEX_URL");

    let config = Config::load_from_dir(temp_dir.path()).unwrap();

    // Should use defaults
    assert_eq!(config.index_url, DEFAULT_INDEX_URL);

    if let Some(v) = orig {
        env::set_var("DX_PY_INDEX_URL", v);
    }
}

/// Test full layering with env vars
#[test]
fn test_full_layering_with_env_vars() {
    use std::env;
    let _guard = ENV_MUTEX.lock().unwrap();

    // Save original env var
    let orig = env::var("DX_PY_MAX_CONCURRENT_DOWNLOADS").ok();

    let default_val = 8usize;
    let global_val = 4usize;
    let project_val = 12usize;
    let env_val = 20usize;

    // Start with defaults
    let mut config = Config::default();
    assert_eq!(config.max_concurrent_downloads, default_val);

    // Apply global config
    let global_config = ConfigFile {
        max_concurrent_downloads: Some(global_val),
        ..Default::default()
    };
    config.merge(&global_config);
    assert_eq!(config.max_concurrent_downloads, global_val);

    // Apply project config
    let project_config = ConfigFile {
        max_concurrent_downloads: Some(project_val),
        ..Default::default()
    };
    config.merge(&project_config);
    assert_eq!(config.max_concurrent_downloads, project_val);

    // Apply env var
    env::set_var("DX_PY_MAX_CONCURRENT_DOWNLOADS", env_val.to_string());
    config.apply_env_vars();
    assert_eq!(config.max_concurrent_downloads, env_val);

    // Restore original
    match orig {
        Some(v) => env::set_var("DX_PY_MAX_CONCURRENT_DOWNLOADS", v),
        None => env::remove_var("DX_PY_MAX_CONCURRENT_DOWNLOADS"),
    }
}
