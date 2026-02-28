//! Generator integration tests.

use dx_workspace::{Generator, Platform, WorkspaceConfig};
use tempfile::tempdir;

#[test]
fn test_vscode_generator() {
    let config = WorkspaceConfig::new("vscode-test");
    let dir = tempdir().unwrap();

    // Change to temp dir for generation
    let output_dir = dir.path();
    let generator = Generator::with_output_dir(&config, output_dir);

    let result = generator.generate(Platform::VsCode);
    assert!(result.is_ok());

    let result = result.unwrap();
    assert!(result.success);
    assert_eq!(result.platform, Platform::VsCode);
    assert!(!result.files.is_empty());

    // Check that .vscode directory was created
    assert!(output_dir.join(".vscode").exists());
}

#[test]
fn test_gitpod_generator() {
    let config = WorkspaceConfig::new("gitpod-test");
    let dir = tempdir().unwrap();

    let output_dir = dir.path();
    let generator = Generator::with_output_dir(&config, output_dir);

    let result = generator.generate(Platform::Gitpod);
    assert!(result.is_ok());

    let result = result.unwrap();
    assert!(result.success);
    assert_eq!(result.platform, Platform::Gitpod);

    // Check that .gitpod.yml was created
    assert!(output_dir.join(".gitpod.yml").exists());
}

#[test]
fn test_codespaces_generator() {
    let config = WorkspaceConfig::new("codespaces-test");
    let dir = tempdir().unwrap();

    let output_dir = dir.path();
    let generator = Generator::with_output_dir(&config, output_dir);

    let result = generator.generate(Platform::Codespaces);
    assert!(result.is_ok());

    let result = result.unwrap();
    assert!(result.success);

    // Check that .devcontainer was created
    assert!(output_dir.join(".devcontainer").exists());
}

#[test]
fn test_nix_flakes_generator() {
    let config = WorkspaceConfig::new("nix-test");
    let dir = tempdir().unwrap();

    let output_dir = dir.path();
    let generator = Generator::with_output_dir(&config, output_dir);

    let result = generator.generate(Platform::NixFlakes);
    assert!(result.is_ok());

    let result = result.unwrap();
    assert!(result.success);

    // Check that flake.nix was created
    assert!(output_dir.join("flake.nix").exists());
}

#[test]
fn test_generate_all() {
    let config = WorkspaceConfig::new("all-platforms-test");
    let dir = tempdir().unwrap();

    let output_dir = dir.path();
    let generator = Generator::with_output_dir(&config, output_dir);

    let results = generator.generate_all();

    // Should have results for all platforms
    assert!(!results.is_empty());

    // Count successes
    let successes = results.iter().filter(|r| r.success).count();
    assert!(successes > 0, "At least some generators should succeed");
}

#[test]
fn test_generate_desktop() {
    let config = WorkspaceConfig::new("desktop-test");
    let dir = tempdir().unwrap();

    let output_dir = dir.path();
    let generator = Generator::with_output_dir(&config, output_dir);

    let results = generator.generate_desktop();

    // Should have results for desktop platforms
    assert!(!results.is_empty());

    // All should be desktop platforms
    for result in &results {
        assert!(result.platform.is_desktop());
    }
}

#[test]
fn test_generate_cloud() {
    let config = WorkspaceConfig::new("cloud-test");
    let dir = tempdir().unwrap();

    let output_dir = dir.path();
    let generator = Generator::with_output_dir(&config, output_dir);

    let results = generator.generate_cloud();

    // Should have results for cloud platforms
    assert!(!results.is_empty());

    // All should be cloud platforms
    for result in &results {
        assert!(result.platform.is_cloud());
    }
}

#[test]
fn test_generator_clean() {
    let config = WorkspaceConfig::new("clean-test");
    let dir = tempdir().unwrap();

    let output_dir = dir.path();
    let generator = Generator::with_output_dir(&config, output_dir);

    // Generate first
    generator.generate(Platform::VsCode).unwrap();
    assert!(output_dir.join(".vscode").exists());

    // Clean
    generator.clean(Platform::VsCode).unwrap();

    // Directory should be gone
    assert!(!output_dir.join(".vscode").exists());
}

#[test]
fn test_generator_exists() {
    let config = WorkspaceConfig::new("exists-test");
    let dir = tempdir().unwrap();

    let output_dir = dir.path();
    let generator = Generator::with_output_dir(&config, output_dir);

    // Should not exist initially
    assert!(!generator.exists(Platform::VsCode));

    // Generate
    generator.generate(Platform::VsCode).unwrap();

    // Should exist now
    assert!(generator.exists(Platform::VsCode));
}

#[test]
fn test_vscode_settings_content() {
    let mut config = WorkspaceConfig::new("settings-test");
    config.editor.tab_size = 2;
    config.editor.insert_spaces = true;
    config.editor.theme = Some("Dracula".into());

    let dir = tempdir().unwrap();
    let output_dir = dir.path();
    let generator = Generator::with_output_dir(&config, output_dir);

    generator.generate(Platform::VsCode).unwrap();

    // Read and verify settings.json
    let settings_path = output_dir.join(".vscode").join("settings.json");
    assert!(settings_path.exists());

    let content = std::fs::read_to_string(&settings_path).unwrap();
    assert!(content.contains("editor.tabSize"));
}

#[test]
fn test_gitpod_yaml_content() {
    let config = WorkspaceConfig::new("gitpod-content-test");
    let dir = tempdir().unwrap();

    let output_dir = dir.path();
    let generator = Generator::with_output_dir(&config, output_dir);

    generator.generate(Platform::Gitpod).unwrap();

    // Read and verify .gitpod.yml
    let gitpod_path = output_dir.join(".gitpod.yml");
    let content = std::fs::read_to_string(&gitpod_path).unwrap();

    // Should contain expected fields
    assert!(content.contains("image:") || content.contains("tasks:"));
}

#[test]
fn test_dx_defaults() {
    use dx_workspace::config::TaskConfig;

    let tasks = TaskConfig::dx_defaults();

    // Should have dx-specific tasks
    assert!(!tasks.tasks.is_empty());

    // Should have build task (check case-insensitively)
    let has_build = tasks
        .tasks
        .iter()
        .any(|t| t.label.to_lowercase().contains("build") || t.command == "dx");
    assert!(has_build, "Should have a build task");
}
