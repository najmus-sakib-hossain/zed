//! Binary format integration tests.

use dx_workspace::{WorkspaceConfig, load_binary, save_binary, validate_binary};
use tempfile::tempdir;

#[test]
fn test_binary_roundtrip_basic() {
    let config = WorkspaceConfig::new("test-project");
    let dir = tempdir().unwrap();
    let path = dir.path().join("workspace.dxws");

    save_binary(&config, &path).unwrap();
    let loaded = load_binary(&path).unwrap();

    assert_eq!(config.name, loaded.name);
    assert_eq!(config.schema_version, loaded.schema_version);
}

#[test]
fn test_binary_roundtrip_full_config() {
    let mut config = WorkspaceConfig::new("dx-full-test");
    config.description = "A comprehensive test project".into();

    // Configure editor settings
    config.editor.tab_size = 2;
    config.editor.insert_spaces = true;
    config.editor.font_family = Some("JetBrains Mono".into());
    config.editor.font_size = Some(14);
    config.editor.theme = Some("One Dark Pro".into());

    // Add some tasks
    use dx_workspace::config::{TaskDefinition, TaskType};
    config.tasks.tasks.push(TaskDefinition {
        label: "Build".into(),
        task_type: TaskType::Shell,
        command: "dx build".into(),
        args: vec!["--release".into()],
        ..Default::default()
    });
    config.tasks.tasks.push(TaskDefinition {
        label: "Dev Server".into(),
        task_type: TaskType::Shell,
        command: "dx dev".into(),
        args: vec![],
        ..Default::default()
    });

    // Add extensions
    use dx_workspace::config::ExtensionInfo;
    config.extensions.core.push(ExtensionInfo {
        id: "rust-lang.rust-analyzer".into(),
        name: "rust-analyzer".into(),
        description: Some("Rust language support".into()),
        required: true,
        platforms: vec![],
    });

    let dir = tempdir().unwrap();
    let path = dir.path().join("full-config.dxws");

    save_binary(&config, &path).unwrap();
    let loaded = load_binary(&path).unwrap();

    assert_eq!(config.name, loaded.name);
    assert_eq!(config.description, loaded.description);
    assert_eq!(config.editor.tab_size, loaded.editor.tab_size);
    assert_eq!(config.editor.font_family, loaded.editor.font_family);
    assert_eq!(config.tasks.tasks.len(), loaded.tasks.tasks.len());
    assert_eq!(config.extensions.core.len(), loaded.extensions.core.len());
}

#[test]
fn test_binary_validation() {
    let config = WorkspaceConfig::new("validation-test");
    let dir = tempdir().unwrap();
    let path = dir.path().join("validate.dxws");

    save_binary(&config, &path).unwrap();
    assert!(validate_binary(&path).unwrap());
}

#[test]
fn test_binary_invalid_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("invalid.dxws");

    // Write garbage data
    std::fs::write(&path, b"not a valid dxws file").unwrap();

    let result = load_binary(&path);
    assert!(result.is_err());
}

#[test]
fn test_binary_nonexistent_file() {
    let result = load_binary("/nonexistent/path/config.dxws");
    assert!(result.is_err());
}

#[test]
fn test_binary_content_hash_stability() {
    let config = WorkspaceConfig::new("hash-test");
    let dir = tempdir().unwrap();

    // Write same config to two files
    let path1 = dir.path().join("file1.dxws");
    let path2 = dir.path().join("file2.dxws");

    save_binary(&config, &path1).unwrap();
    save_binary(&config, &path2).unwrap();

    // Content hashes should be identical
    let hash1 = dx_workspace::binary::get_content_hash(&path1).unwrap();
    let hash2 = dx_workspace::binary::get_content_hash(&path2).unwrap();

    assert_eq!(hash1, hash2);
}

#[test]
fn test_binary_different_configs_different_hashes() {
    let config1 = WorkspaceConfig::new("project-one");
    let config2 = WorkspaceConfig::new("project-two");
    let dir = tempdir().unwrap();

    let path1 = dir.path().join("file1.dxws");
    let path2 = dir.path().join("file2.dxws");

    save_binary(&config1, &path1).unwrap();
    save_binary(&config2, &path2).unwrap();

    let hash1 = dx_workspace::binary::get_content_hash(&path1).unwrap();
    let hash2 = dx_workspace::binary::get_content_hash(&path2).unwrap();

    assert_ne!(hash1, hash2);
}
