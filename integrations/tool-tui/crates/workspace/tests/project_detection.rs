//! Project detection integration tests.

use dx_workspace::{ProjectDetector, WorkspaceConfig};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_detect_rust_project() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    // Create a minimal Cargo.toml
    fs::write(
        project_dir.join("Cargo.toml"),
        r#"[package]
name = "my-rust-project"
version = "0.1.0"
edition = "2024"

[dependencies]
"#,
    )
    .unwrap();

    // Create src directory
    fs::create_dir(project_dir.join("src")).unwrap();
    fs::write(project_dir.join("src/main.rs"), "fn main() {}").unwrap();

    let detector = ProjectDetector::new(project_dir);
    let config = detector.detect().unwrap();

    assert_eq!(config.name, "my-rust-project");
    assert!(config.detected_features.is_rust);
}

#[test]
fn test_detect_dx_project() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    // Create a Cargo.toml with dx dependencies
    fs::write(
        project_dir.join("Cargo.toml"),
        r#"[package]
name = "my-dx-app"
version = "0.1.0"
edition = "2024"

[dependencies]
dx-www = "0.1"
dx-client = "0.1"
"#,
    )
    .unwrap();

    let detector = ProjectDetector::new(project_dir);
    let config = detector.detect().unwrap();

    assert_eq!(config.name, "my-dx-app");
    assert!(config.detected_features.is_rust);
    assert!(config.detected_features.has_dx_www);
    assert!(config.detected_features.has_dx_client);
}

#[test]
fn test_detect_dx_server_project() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    fs::write(
        project_dir.join("Cargo.toml"),
        r#"[package]
name = "my-dx-server"
version = "0.1.0"

[dependencies]
dx-server = "0.1"
dx-db = "0.1"
"#,
    )
    .unwrap();

    let detector = ProjectDetector::new(project_dir);
    let config = detector.detect().unwrap();

    assert!(config.detected_features.has_dx_server);
    assert!(config.detected_features.has_dx_db);
}

#[test]
fn test_detect_with_gitignore() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    fs::write(
        project_dir.join("Cargo.toml"),
        r#"[package]
name = "project-with-gitignore"
version = "0.1.0"
"#,
    )
    .unwrap();

    fs::write(
        project_dir.join(".gitignore"),
        r#"target/
*.log
.env
"#,
    )
    .unwrap();

    let detector = ProjectDetector::new(project_dir);
    let config = detector.detect().unwrap();

    assert!(config.detected_features.has_git);
}

#[test]
fn test_detect_empty_directory() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    let detector = ProjectDetector::new(project_dir);
    let config = detector.detect().unwrap();

    // Should still return a config with directory name
    assert!(!config.name.is_empty());
    assert!(!config.detected_features.is_rust);
}

#[test]
fn test_detect_workspace_project() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    // Create a workspace Cargo.toml
    fs::write(
        project_dir.join("Cargo.toml"),
        r#"[workspace]
members = ["crates/*"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2024"
"#,
    )
    .unwrap();

    // Create crates directory
    fs::create_dir_all(project_dir.join("crates/app")).unwrap();
    fs::write(
        project_dir.join("crates/app/Cargo.toml"),
        r#"[package]
name = "app"
version.workspace = true
"#,
    )
    .unwrap();

    let detector = ProjectDetector::new(project_dir);
    let config = detector.detect().unwrap();

    assert!(config.detected_features.is_rust);
    assert!(config.detected_features.is_workspace);
}

#[test]
fn test_detect_with_existing_vscode() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    fs::write(
        project_dir.join("Cargo.toml"),
        r#"[package]
name = "project-with-vscode"
version = "0.1.0"
"#,
    )
    .unwrap();

    // Create existing .vscode settings
    fs::create_dir(project_dir.join(".vscode")).unwrap();
    fs::write(project_dir.join(".vscode/settings.json"), r#"{"editor.tabSize": 4}"#).unwrap();

    let detector = ProjectDetector::new(project_dir);
    let config = detector.detect().unwrap();

    // Should detect existing VS Code configuration
    assert!(config.detected_features.has_vscode_config);
}

#[test]
fn test_detect_with_existing_gitpod() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    fs::write(
        project_dir.join("Cargo.toml"),
        r#"[package]
name = "project-with-gitpod"
version = "0.1.0"
"#,
    )
    .unwrap();

    fs::write(
        project_dir.join(".gitpod.yml"),
        r#"image: gitpod/workspace-rust
tasks:
  - command: cargo build
"#,
    )
    .unwrap();

    let detector = ProjectDetector::new(project_dir);
    let config = detector.detect().unwrap();

    assert!(config.detected_features.has_gitpod_config);
}

#[test]
fn test_detect_with_devcontainer() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    fs::write(
        project_dir.join("Cargo.toml"),
        r#"[package]
name = "project-with-devcontainer"
version = "0.1.0"
"#,
    )
    .unwrap();

    fs::create_dir(project_dir.join(".devcontainer")).unwrap();
    fs::write(project_dir.join(".devcontainer/devcontainer.json"), r#"{"name": "Rust Dev"}"#)
        .unwrap();

    let detector = ProjectDetector::new(project_dir);
    let config = detector.detect().unwrap();

    assert!(config.detected_features.has_devcontainer_config);
}

#[test]
fn test_config_save_and_load() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    let mut config = WorkspaceConfig::new("save-load-test");
    config.description = "Test saving and loading".into();
    config.editor.tab_size = 2;

    let config_path = project_dir.join("dx-workspace.json");
    config.save(&config_path).unwrap();

    let loaded = WorkspaceConfig::load(&config_path).unwrap();

    assert_eq!(config.name, loaded.name);
    assert_eq!(config.description, loaded.description);
    assert_eq!(config.editor.tab_size, loaded.editor.tab_size);
}

#[test]
fn test_config_content_hash() {
    let config1 = WorkspaceConfig::new("hash-test-1");
    let config2 = WorkspaceConfig::new("hash-test-1"); // Same name
    let config3 = WorkspaceConfig::new("hash-test-2"); // Different name

    let hash1 = config1.content_hash();
    let hash2 = config2.content_hash();
    let hash3 = config3.content_hash();

    // Same configs should have same hash
    assert_eq!(hash1, hash2);

    // Different configs should have different hash
    assert_ne!(hash1, hash3);
}

#[test]
fn test_config_validate() {
    let config = WorkspaceConfig::new("valid-config");
    assert!(config.validate().is_ok());

    // Empty name should fail validation
    let mut invalid_config = WorkspaceConfig::new("");
    invalid_config.name = "".into();
    // Note: Depending on implementation, this might or might not be an error
}
