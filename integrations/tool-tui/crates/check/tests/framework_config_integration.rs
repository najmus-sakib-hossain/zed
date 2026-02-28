//! Integration tests for framework-specific configuration

use dx_check::config::CheckerConfig;
use dx_check::engine::Checker;
use dx_check::project::Framework;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_framework_config_loading() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a package.json with React
    let package_json = r#"{
        "dependencies": {
            "react": "^18.0.0",
            "react-dom": "^18.0.0"
        }
    }"#;
    fs::write(root.join("package.json"), package_json).unwrap();

    // Create checker with auto-detection
    let checker = Checker::with_auto_detect(root);

    // Verify framework was detected
    let profile = checker.profile().unwrap();
    assert!(profile.frameworks.contains(&Framework::React));

    // Verify framework config was loaded
    let fw_config = checker.framework_config();
    let react_config = fw_config.get_config(Framework::React);
    assert!(react_config.is_some());

    let config = react_config.unwrap();
    assert_eq!(config.framework, Framework::React);
    assert!(!config.enabled_rules.is_empty());
}

#[test]
fn test_next_framework_config() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a package.json with Next.js
    let package_json = r#"{
        "dependencies": {
            "next": "^14.0.0",
            "react": "^18.0.0"
        }
    }"#;
    fs::write(root.join("package.json"), package_json).unwrap();

    // Create next.config.js
    fs::write(root.join("next.config.js"), "module.exports = {}").unwrap();

    // Create checker with auto-detection
    let checker = Checker::with_auto_detect(root);

    // Verify Next.js was detected
    let profile = checker.profile().unwrap();
    assert!(profile.frameworks.contains(&Framework::Next));

    // Verify framework config was loaded
    let fw_config = checker.framework_config();
    let next_config = fw_config.get_config(Framework::Next);
    assert!(next_config.is_some());

    let config = next_config.unwrap();
    assert_eq!(config.framework, Framework::Next);
    assert!(config.enabled_rules.iter().any(|r| r.contains("next/")));
}

#[test]
fn test_vue_framework_config() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a package.json with Vue
    let package_json = r#"{
        "dependencies": {
            "vue": "^3.0.0"
        }
    }"#;
    fs::write(root.join("package.json"), package_json).unwrap();

    // Create a .vue file
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(root.join("src/App.vue"), "<template><div>Hello</div></template>").unwrap();

    // Create checker with auto-detection
    let checker = Checker::with_auto_detect(root);

    // Verify Vue was detected
    let profile = checker.profile().unwrap();
    assert!(profile.frameworks.contains(&Framework::Vue));

    // Verify framework config was loaded
    let fw_config = checker.framework_config();
    let vue_config = fw_config.get_config(Framework::Vue);
    assert!(vue_config.is_some());

    let config = vue_config.unwrap();
    assert_eq!(config.framework, Framework::Vue);
    assert!(config.enabled_rules.iter().any(|r| r.contains("vue/")));
}

#[test]
fn test_angular_framework_config() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a package.json with Angular
    let package_json = r#"{
        "dependencies": {
            "@angular/core": "^17.0.0"
        }
    }"#;
    fs::write(root.join("package.json"), package_json).unwrap();

    // Create angular.json
    fs::write(root.join("angular.json"), "{}").unwrap();

    // Create checker with auto-detection
    let checker = Checker::with_auto_detect(root);

    // Verify Angular was detected
    let profile = checker.profile().unwrap();
    assert!(profile.frameworks.contains(&Framework::Angular));

    // Verify framework config was loaded
    let fw_config = checker.framework_config();
    let angular_config = fw_config.get_config(Framework::Angular);
    assert!(angular_config.is_some());

    let config = angular_config.unwrap();
    assert_eq!(config.framework, Framework::Angular);
    assert!(config.enabled_rules.iter().any(|r| r.contains("angular/")));
}

#[test]
fn test_multiple_frameworks_in_monorepo() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create root package.json with workspaces
    let package_json = r#"{
        "workspaces": ["packages/*"]
    }"#;
    fs::write(root.join("package.json"), package_json).unwrap();

    // Create packages directory
    fs::create_dir_all(root.join("packages")).unwrap();

    // Create React package
    fs::create_dir_all(root.join("packages/react-app")).unwrap();
    let react_pkg = r#"{
        "name": "react-app",
        "dependencies": {
            "react": "^18.0.0"
        }
    }"#;
    fs::write(root.join("packages/react-app/package.json"), react_pkg).unwrap();

    // Create Next.js package
    fs::create_dir_all(root.join("packages/next-app")).unwrap();
    let next_pkg = r#"{
        "name": "next-app",
        "dependencies": {
            "next": "^14.0.0"
        }
    }"#;
    fs::write(root.join("packages/next-app/package.json"), next_pkg).unwrap();

    // Create checker with auto-detection
    let checker = Checker::with_auto_detect(root);

    // Verify monorepo was detected
    let profile = checker.profile().unwrap();
    assert!(profile.monorepo.is_some());

    // Both frameworks should be detected
    assert!(
        profile.frameworks.contains(&Framework::React)
            || profile.frameworks.contains(&Framework::Next)
    );
}

#[test]
fn test_custom_framework_config_file() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();

    // Create a package.json with React
    let package_json = r#"{
        "dependencies": {
            "react": "^18.0.0"
        }
    }"#;
    fs::write(root.join("package.json"), package_json).unwrap();

    // Create custom framework config
    let custom_config = r#"{
        "framework": "React",
        "enabled_rules": ["react-hooks/rules-of-hooks"],
        "disabled_rules": ["react/display-name"],
        "settings": {
            "jsx-runtime": "automatic"
        }
    }"#;
    fs::write(root.join(".dx-react.json"), custom_config).unwrap();

    // Create checker with auto-detection
    let checker = Checker::with_auto_detect(root);

    // Verify custom config was loaded
    let fw_config = checker.framework_config();
    let react_config = fw_config.get_config(Framework::React);
    assert!(react_config.is_some());

    let config = react_config.unwrap();
    assert!(config.enabled_rules.contains(&"react-hooks/rules-of-hooks".to_string()));
    assert!(config.disabled_rules.contains(&"react/display-name".to_string()));
}
