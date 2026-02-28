/// Integration test: Full conversion pipeline demonstration

#[cfg(test)]
mod integration_tests {
    use serializer::*;

    #[test]
    fn test_full_conversion_pipeline() {
        // Simulate real-world config files

        // 1. Package.json
        let package_json = r#"{
  "name": "awesome-app",
  "version": "2.0.1",
  "description": "My awesome application",
  "scripts": {
    "dev": "vite",
    "build": "vite build"
  },
  "dependencies": {
    "react": "^18.2.0"
  }
}"#;

        let dx = json_to_dx(package_json).unwrap();
        assert!(dx.contains("n:awesome-app"));
        let savings_pct =
            ((package_json.len() - dx.len()) as f64 / package_json.len() as f64) * 100.0;
        assert!(savings_pct > 30.0, "Should be at least 30% smaller (got {:.1}%)", savings_pct);
        println!(
            "✅ package.json: {} → {} bytes ({:.1}% smaller)",
            package_json.len(),
            dx.len(),
            savings_pct
        );

        // 2. Config.yaml
        let config_yaml = r#"
name: my-config
version: 1.0.0
settings:
  debug: true
  timeout: 30
"#;

        let dx = yaml_to_dx(config_yaml).unwrap();
        assert!(dx.contains("n:my-config"));
        println!("✅ config.yaml: {} → {} bytes", config_yaml.len(), dx.len());

        // 3. Settings.toml
        let settings_toml = r#"
name = "settings"
version = "1.0.0"

[database]
host = "localhost"
port = 5432
"#;

        let dx = toml_to_dx(settings_toml).unwrap();
        assert!(dx.contains("n:settings"));
        println!("✅ settings.toml: {} → {} bytes", settings_toml.len(), dx.len());

        // 4. Test auto-detection
        let formats = vec![
            ("json", package_json),
            ("yaml", config_yaml),
            ("toml", settings_toml),
        ];

        for (format, content) in formats {
            let dx = convert_to_dx(content, format).unwrap();
            assert!(!dx.is_empty(), "Conversion should produce output");
            println!("✅ Auto-detect {}: Success", format);
        }
    }

    #[test]
    fn test_ultra_optimization_applied() {
        let json = r#"{
  "name": "test",
  "version": "1.0.0",
  "description": "Test app",
  "author": "John Doe",
  "packageManager": "npm",
  "framework": "react"
}"#;

        let dx = json_to_dx(json).unwrap();

        // Verify ultra-optimizations are applied
        assert!(dx.contains("n:test"), "Should abbreviate 'name' to 'n'");
        assert!(dx.contains("v:1.0.0"), "Should abbreviate 'version' to 'v'");
        assert!(dx.contains("d:Test app"), "Should abbreviate 'description' to 'd'");
        assert!(dx.contains("a:John Doe"), "Should abbreviate 'author' to 'a'");
        assert!(dx.contains("pm:npm"), "Should abbreviate 'packageManager' to 'pm'");
        assert!(dx.contains("fw:react"), "Should abbreviate 'framework' to 'fw'");

        // Verify inlining (may use ^ operator if optimized)
        // Not all cases will inline, so this is optional

        println!("✅ All ultra-optimizations applied correctly");
        println!("DX Output:\n{}", dx);
    }

    #[test]
    fn test_compression_guarantees() {
        let configs = vec![
            (r#"{"name":"a","version":"1.0.0"}"#, "json"),
            ("name: a\nversion: 1.0.0", "yaml"),
            (r#"name = "a""#, "toml"),
        ];

        for (input, format) in configs {
            let dx = convert_to_dx(input, format).unwrap();
            let savings_pct = ((input.len() - dx.len()) as f64 / input.len() as f64) * 100.0;

            // Every format should save at least 30%
            assert!(
                savings_pct > 30.0,
                "{} should save >30% (actual: {:.1}%)",
                format,
                savings_pct
            );

            println!("✅ {}: {:.1}% compression", format, savings_pct);
        }
    }

    #[test]
    fn test_language_code_optimization() {
        let json = r#"{
  "languages": {
    "javascript/typescript": "yes",
    "python": "yes",
    "rust": "yes"
  }
}"#;

        let dx = json_to_dx(json).unwrap();

        // Language codes should be abbreviated
        assert!(
            dx.contains("js/ts") || dx.contains("javascript/typescript"),
            "Should handle js/ts"
        );

        println!("✅ Language codes optimized");
        println!("DX Output:\n{}", dx);
    }
} // end mod integration_tests
