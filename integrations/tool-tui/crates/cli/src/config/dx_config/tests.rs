//! Configuration tests

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::utils::error::DxError;
    use proptest::prelude::*;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_temp_config(content: &str) -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let config_path = dir.path().join("dx.toml");
        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        (dir, config_path)
    }

    #[test]
    fn test_load_valid_config() {
        let content = r#"
[project]
name = "test-project"
version = "1.0.0"
description = "A test project"

[build]
target = "node"
minify = false

[dev]
port = 8080
"#;
        let (_dir, path) = create_temp_config(content);
        let config = DxConfig::load(&path).unwrap();

        assert_eq!(config.project.name, "test-project");
        assert_eq!(config.project.version, "1.0.0");
        assert_eq!(config.build.target, "node");
        assert!(!config.build.minify);
        assert_eq!(config.dev.port, 8080);
    }

    #[test]
    fn test_load_minimal_config() {
        let content = r#"
[project]
name = "minimal"
"#;
        let (_dir, path) = create_temp_config(content);
        let config = DxConfig::load(&path).unwrap();

        assert_eq!(config.project.name, "minimal");
        assert_eq!(config.project.version, "0.1.0");
        assert_eq!(config.build.target, "browser");
        assert_eq!(config.dev.port, 3000);
    }

    #[test]
    fn test_load_config_not_found() {
        let result = DxConfig::load(std::path::Path::new("nonexistent.toml"));
        assert!(result.is_err());
        match result.unwrap_err() {
            DxError::ConfigNotFound { path } => {
                assert_eq!(path, PathBuf::from("nonexistent.toml"));
            }
            _ => panic!("Expected ConfigNotFound error"),
        }
    }

    #[test]
    fn test_load_invalid_config() {
        let content = r#"
[project]
name = "test"
version = 123
"#;
        let (_dir, path) = create_temp_config(content);
        let result = DxConfig::load(&path);

        assert!(result.is_err());
        match result.unwrap_err() {
            DxError::ConfigInvalid { line, message, .. } => {
                assert!(line > 0);
                assert!(!message.is_empty());
            }
            _ => panic!("Expected ConfigInvalid error"),
        }
    }

    #[test]
    fn test_config_with_tools() {
        let content = r#"
[project]
name = "with-tools"

[tools.style]
preprocessor = "sass"
modules = true
postcss_plugins = ["autoprefixer"]

[tools.media]
quality = 90
formats = ["webp", "avif"]
"#;
        let (_dir, path) = create_temp_config(content);
        let config = DxConfig::load(&path).unwrap();

        let style = config.tools.style.unwrap();
        assert_eq!(style.preprocessor, Some("sass".to_string()));
        assert!(style.modules);
        assert_eq!(style.postcss_plugins, vec!["autoprefixer"]);

        let media = config.tools.media.unwrap();
        assert_eq!(media.quality, 90);
        assert_eq!(media.formats, vec!["webp", "avif"]);
    }

    #[test]
    fn test_cache_path_generation() {
        let config_path = std::path::Path::new("/project/dx.toml");
        let cache_path = cache::cache_path(config_path);
        assert_eq!(cache_path, PathBuf::from("/project/.dx.toml.cache"));
    }

    #[test]
    fn test_config_caching() {
        let content = r#"
[project]
name = "cached-project"
version = "2.0.0"
"#;
        let (_dir, path) = create_temp_config(content);

        let config1 = DxConfig::load(&path).unwrap();
        assert_eq!(config1.project.name, "cached-project");

        let cache_path = cache::cache_path(&path);
        assert!(cache_path.exists());

        let config2 = DxConfig::load(&path).unwrap();
        assert_eq!(config2.project.name, "cached-project");
        assert_eq!(config1, config2);
    }

    #[test]
    fn test_cache_invalidation_on_source_change() {
        let content1 = r#"
[project]
name = "original"
"#;
        let (dir, path) = create_temp_config(content1);

        let config1 = DxConfig::load(&path).unwrap();
        assert_eq!(config1.project.name, "original");

        DxConfig::invalidate_cache(&path);

        let content2 = r#"
[project]
name = "modified"
"#;
        fs::write(&path, content2).unwrap();

        let config2 = DxConfig::load(&path).unwrap();
        assert_eq!(config2.project.name, "modified");

        drop(dir);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        #[test]
        fn prop_custom_config_path_override(
            name in "[a-zA-Z][a-zA-Z0-9_-]{0,20}",
            version in "[0-9]{1,2}\\.[0-9]{1,2}\\.[0-9]{1,2}"
        ) {
            let content = format!(r#"
[project]
name = "{}"
version = "{}"
"#, name, version);

            let dir = TempDir::new().unwrap();
            let custom_path = dir.path().join("custom-config.toml");
            fs::write(&custom_path, &content).unwrap();

            let config = DxConfig::load_with_override(Some(&custom_path)).unwrap();

            prop_assert_eq!(config.project.name, name);
            prop_assert_eq!(config.project.version, version);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10))]

        #[test]
        fn prop_invalid_config_error_reporting(
            name in "[a-zA-Z][a-zA-Z0-9_-]{0,20}",
            invalid_line in 2usize..10
        ) {
            let mut lines: Vec<String> = vec![
                "[project]".to_string(),
                format!("name = \"{}\"", name),
            ];

            for _ in 2..invalid_line {
                lines.push("# comment".to_string());
            }

            lines.push("version = \"unclosed".to_string());

            let content = lines.join("\n");
            let (_dir, path) = create_temp_config(&content);

            let result = DxConfig::load(&path);
            prop_assert!(result.is_err());

            match result.unwrap_err() {
                DxError::ConfigInvalid { path: err_path, line, message } => {
                    prop_assert_eq!(err_path, path);
                    prop_assert!(line > 0);
                    prop_assert!(!message.is_empty());
                }
                other => prop_assert!(false, "Expected ConfigInvalid, got {:?}", other),
            }
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        #[test]
        fn prop_config_cache_round_trip(
            name in "[a-zA-Z][a-zA-Z0-9_-]{0,20}",
            version in "[0-9]{1,2}\\.[0-9]{1,2}\\.[0-9]{1,2}",
            port in 1024u16..65535,
            minify in proptest::bool::ANY
        ) {
            let content = format!(r#"
[project]
name = "{}"
version = "{}"

[build]
minify = {}

[dev]
port = {}
"#, name, version, minify, port);

            let (_dir, path) = create_temp_config(&content);

            let config1 = DxConfig::load(&path).unwrap();

            let cache_path = cache::cache_path(&path);
            prop_assert!(cache_path.exists());

            let config2 = DxConfig::load(&path).unwrap();

            prop_assert_eq!(&config1.project.name, &config2.project.name);
            prop_assert_eq!(&config1.project.version, &config2.project.version);
            prop_assert_eq!(config1.build.minify, config2.build.minify);
            prop_assert_eq!(config1.dev.port, config2.dev.port);
            prop_assert_eq!(&config1, &config2);
        }
    }

    #[test]
    fn test_validate_empty_project_name() {
        let config = DxConfig {
            project: ProjectConfig {
                name: "".to_string(),
                version: "1.0.0".to_string(),
                description: None,
            },
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_zero_port() {
        let config = DxConfig {
            project: ProjectConfig {
                name: "test".to_string(),
                version: "1.0.0".to_string(),
                description: None,
            },
            dev: DevConfig {
                port: 0,
                open: false,
                https: false,
            },
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_invalid_media_quality() {
        let config = DxConfig {
            project: ProjectConfig {
                name: "test".to_string(),
                version: "1.0.0".to_string(),
                description: None,
            },
            tools: ToolsConfig {
                media: Some(MediaToolConfig {
                    quality: 0,
                    formats: vec![],
                }),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_save_atomic_creates_backup() {
        let content = r#"
[project]
name = "original"
version = "1.0.0"
"#;
        let (_dir, path) = create_temp_config(content);

        let new_config = DxConfig {
            project: ProjectConfig {
                name: "modified".to_string(),
                version: "2.0.0".to_string(),
                description: None,
            },
            ..Default::default()
        };

        new_config.save_atomic(&path).unwrap();

        let backup_path = path.with_extension("toml.bak");
        assert!(backup_path.exists());

        let backup_content = fs::read_to_string(&backup_path).unwrap();
        assert!(backup_content.contains("original"));

        let new_content = fs::read_to_string(&path).unwrap();
        assert!(new_content.contains("modified"));
    }

    #[test]
    fn test_check_unknown_fields() {
        let content = r#"
[project]
name = "test"

[unknown_section]
foo = "bar"
"#;
        let unknown = DxConfig::check_unknown_fields(content);
        assert!(!unknown.is_empty());
        assert!(unknown.iter().any(|s| s.contains("unknown_section")));
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_config_field_validation_port(port in 1u16..65535) {
            let content = format!(r#"
[project]
name = "test"

[dev]
port = {}
"#, port);

            let (_dir, path) = create_temp_config(&content);
            let result = DxConfig::load_validated(&path);

            prop_assert!(result.is_ok(), "Port {} should be valid", port);
        }

        #[test]
        fn prop_config_field_validation_quality(quality in 1u8..=100) {
            let content = format!(r#"
[project]
name = "test"

[tools.media]
quality = {}
"#, quality);

            let (_dir, path) = create_temp_config(&content);
            let result = DxConfig::load_validated(&path);

            prop_assert!(result.is_ok(), "Quality {} should be valid", quality);
        }

        #[test]
        fn prop_config_field_validation_invalid_quality(quality in 101u8..=255) {
            let content = format!(r#"
[project]
name = "test"

[tools.media]
quality = {}
"#, quality);

            let (_dir, path) = create_temp_config(&content);
            let result = DxConfig::load_validated(&path);

            prop_assert!(result.is_err(), "Quality {} should be invalid", quality);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        #[test]
        fn prop_config_backup_on_save(
            original_name in "[a-zA-Z][a-zA-Z0-9_-]{1,20}",
            new_name in "[a-zA-Z][a-zA-Z0-9_-]{1,20}"
        ) {
            let content = format!(r#"
[project]
name = "{}"
"#, original_name);

            let (_dir, path) = create_temp_config(&content);

            let new_config = DxConfig {
                project: ProjectConfig {
                    name: new_name.clone(),
                    version: "1.0.0".to_string(),
                    description: None,
                },
                ..Default::default()
            };

            new_config.save_atomic(&path).unwrap();

            let backup_path = path.with_extension("toml.bak");
            prop_assert!(backup_path.exists());

            let backup_content = fs::read_to_string(&backup_path).unwrap();
            prop_assert!(backup_content.contains(&original_name));

            let new_content = fs::read_to_string(&path).unwrap();
            prop_assert!(new_content.contains(&new_name));
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10))]

        #[test]
        fn prop_cache_invalidation_on_source_change(
            name1 in "[a-zA-Z][a-zA-Z0-9_-]{1,20}",
            name2 in "[a-zA-Z][a-zA-Z0-9_-]{1,20}"
        ) {
            let content1 = format!(r#"
[project]
name = "{}"
"#, name1);

            let (_dir, path) = create_temp_config(&content1);

            let config1 = DxConfig::load(&path).unwrap();
            prop_assert_eq!(&config1.project.name, &name1);

            DxConfig::invalidate_cache(&path);

            let content2 = format!(r#"
[project]
name = "{}"
"#, name2);
            fs::write(&path, &content2).unwrap();

            let config2 = DxConfig::load(&path).unwrap();
            prop_assert_eq!(&config2.project.name, &name2);
        }
    }
}
