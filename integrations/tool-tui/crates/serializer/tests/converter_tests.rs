/// Test the converters from various formats to DX ULTRA

#[cfg(test)]
mod tests {
    use serializer::{convert_to_dx, json_to_dx, toml_to_dx, yaml_to_dx};

    const TEST_JSON: &str = r#"{
    "name": "test-app",
    "version": "1.0.0",
    "description": "A test application",
    "author": "Test Author",
    "languages": [
        {
            "name": "javascript/typescript",
            "runtime": "bun",
            "compiler": "tsc",
            "bundler": "vite",
            "packageManager": "bun",
            "framework": "react"
        },
        {
            "name": "python",
            "runtime": "cpython",
            "packageManager": "uv",
            "framework": "django"
        }
    ],
    "workspace": ["frontend/www", "frontend/mobile"],
    "style": {
        "path": "@/style",
        "engine": ["automic", "enhanced"],
        "themes": ["dx", "vercel"]
    }
}"#;

    #[test]
    fn test_json_conversion() {
        let result = json_to_dx(TEST_JSON);
        assert!(result.is_ok(), "JSON conversion failed: {:?}", result.err());

        let dx = result.unwrap();
        println!("JSON → DX ULTRA:\n{}", dx);

        // Verify optimizations
        assert!(dx.contains("n:test-app"), "Should abbreviate 'name' to 'n'");
        assert!(dx.contains("v:1.0.0"), "Should abbreviate 'version' to 'v'");
        assert!(dx.contains("d:A test application"), "Should abbreviate 'description' to 'd'");
        assert!(dx.len() < TEST_JSON.len(), "DX should be smaller than JSON");
    }

    #[test]
    fn test_yaml_conversion() {
        let yaml = r#"
name: test-app
version: 1.0.0
description: A test application
workspace:
  - frontend/www
  - frontend/mobile
"#;

        let result = yaml_to_dx(yaml);
        assert!(result.is_ok(), "YAML conversion failed: {:?}", result.err());

        let dx = result.unwrap();
        println!("YAML → DX ULTRA:\n{}", dx);
        assert!(dx.contains("n:test-app"));
    }

    #[test]
    fn test_toml_conversion() {
        let toml = r#"
name = "test-app"
version = "1.0.0"
description = "A test application"
workspace = ["frontend/www", "frontend/mobile"]
"#;

        let result = toml_to_dx(toml);
        assert!(result.is_ok(), "TOML conversion failed: {:?}", result.err());

        let dx = result.unwrap();
        println!("TOML → DX ULTRA:\n{}", dx);
        assert!(dx.contains("n:test-app"));
    }

    #[test]
    fn test_auto_detect_format() {
        // Test JSON
        let dx_from_json = convert_to_dx(TEST_JSON, "json").unwrap();
        assert!(dx_from_json.contains("n:test-app"));

        // Test YAML
        let yaml = "name: test\nversion: 1.0.0";
        let dx_from_yaml = convert_to_dx(yaml, "yaml").unwrap();
        assert!(dx_from_yaml.contains("n:test"));
    }

    #[test]
    fn test_optimization_quality() {
        let result = json_to_dx(TEST_JSON);
        assert!(result.is_ok());

        let dx = result.unwrap();
        let json_size = TEST_JSON.len();
        let dx_size = dx.len();

        let savings = ((json_size - dx_size) as f64 / json_size as f64) * 100.0;

        println!("\n=== OPTIMIZATION RESULTS ===");
        println!("JSON size: {} bytes", json_size);
        println!("DX size:   {} bytes", dx_size);
        println!("Savings:   {:.1}% smaller!", savings);

        assert!(savings > 50.0, "Should save at least 50% vs JSON");
    }
}
