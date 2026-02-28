/// Comprehensive test to verify all converters work correctly

#[cfg(test)]
mod verification_tests {
    use serializer::*;

    #[test]
    fn verify_json_to_dx_ultra() {
        println!("\n=== JSON → DX ULTRA ===");

        let json = r#"{
  "name": "test-app",
  "version": "1.0.0",
  "description": "Test application",
  "author": "John Doe",
  "license": "MIT",
  "packageManager": "npm",
  "framework": "react"
}"#;

        let dx = json_to_dx(json).expect("JSON conversion should work");
        println!("Input ({} bytes):\n{}", json.len(), json);
        println!("\nOutput ({} bytes):\n{}", dx.len(), dx);

        // Verify all optimizations are applied
        assert!(dx.contains("n:"), "Should have 'n:' for name");
        assert!(dx.contains("v:"), "Should have 'v:' for version");
        assert!(dx.contains("d:"), "Should have 'd:' for description");
        assert!(dx.contains("a:"), "Should have 'a:' for author");
        assert!(dx.contains("lic:"), "Should have 'lic:' for license");
        assert!(dx.contains("pm:"), "Should have 'pm:' for packageManager");
        assert!(dx.contains("fw:"), "Should have 'fw:' for framework");

        let compression = ((json.len() - dx.len()) as f64 / json.len() as f64) * 100.0;
        println!("Compression: {:.1}%", compression);
        assert!(compression > 30.0, "Should compress at least 30%");
    }

    #[test]
    fn verify_yaml_to_dx_ultra() {
        println!("\n=== YAML → DX ULTRA ===");

        let yaml = r#"
name: test-app
version: 1.0.0
description: Test application
author: John Doe
packageManager: npm
framework: react
"#;

        let dx = yaml_to_dx(yaml).expect("YAML conversion should work");
        println!("Input ({} bytes):\n{}", yaml.len(), yaml);
        println!("\nOutput ({} bytes):\n{}", dx.len(), dx);

        // Verify optimizations
        assert!(dx.contains("n:test-app"), "Should have optimized name");
        assert!(dx.contains("v:1.0.0"), "Should have optimized version");
        assert!(dx.contains("pm:npm"), "Should have optimized packageManager");

        let compression = ((yaml.len() - dx.len()) as f64 / yaml.len() as f64) * 100.0;
        println!("Compression: {:.1}%", compression);
    }

    #[test]
    fn verify_toml_to_dx_ultra() {
        println!("\n=== TOML → DX ULTRA ===");

        let toml = r#"
name = "test-app"
version = "1.0.0"
description = "Test application"
author = "John Doe"

[dependencies]
react = "^18.0.0"
"#;

        let dx = toml_to_dx(toml).expect("TOML conversion should work");
        println!("Input ({} bytes):\n{}", toml.len(), toml);
        println!("\nOutput ({} bytes):\n{}", dx.len(), dx);

        // Verify optimizations
        assert!(dx.contains("n:test-app"), "Should have optimized name");
        assert!(dx.contains("v:1.0.0"), "Should have optimized version");

        let compression = ((toml.len() - dx.len()) as f64 / toml.len() as f64) * 100.0;
        println!("Compression: {:.1}%", compression);
    }

    #[test]
    fn verify_complex_json_with_arrays() {
        println!("\n=== Complex JSON with Arrays ===");

        let json = r#"{
  "name": "complex-app",
  "version": "2.0.0",
  "workspace": ["frontend", "backend", "shared"],
  "languages": {
    "javascript/typescript": true,
    "python": true,
    "rust": true
  },
  "dependencies": {
    "react": "^18.0.0",
    "vue": "^3.0.0"
  }
}"#;

        let dx = json_to_dx(json).expect("Complex JSON should work");
        println!("Input ({} bytes):\n{}", json.len(), json);
        println!("\nOutput ({} bytes):\n{}", dx.len(), dx);

        // Verify array handling
        assert!(dx.contains("|") || dx.contains(">"), "Should have array separator");

        // Verify language code optimization
        let has_lang_opt = dx.contains("js/ts") || dx.contains("py") || dx.contains("rs");
        assert!(has_lang_opt, "Should optimize language codes");

        let compression = ((json.len() - dx.len()) as f64 / json.len() as f64) * 100.0;
        println!("Compression: {:.1}%", compression);
        assert!(compression > 30.0, "Should compress at least 30%");
    }

    #[test]
    fn verify_dx_ultra_optimization_completeness() {
        println!("\n=== Optimization Completeness Test ===");

        // Test all major optimization rules
        let json = r#"{
  "name": "app",
  "version": "1.0.0",
  "description": "My app",
  "author": "Dev",
  "license": "MIT",
  "packageManager": "bun",
  "framework": "react",
  "runtime": "node",
  "compiler": "tsc",
  "bundler": "vite"
}"#;

        let dx = json_to_dx(json).expect("Should convert");
        println!("DX Output:\n{}", dx);

        // Count how many optimizations were applied
        let optimizations = [
            ("n:", "name"),
            ("v:", "version"),
            ("d:", "description"),
            ("a:", "author"),
            ("lic:", "license"),
            ("pm:", "packageManager"),
            ("fw:", "framework"),
            ("rt:", "runtime"),
            ("cp:", "compiler"),
            ("bd:", "bundler"),
        ];

        let mut applied = 0;
        for (opt, _) in &optimizations {
            if dx.contains(opt) {
                applied += 1;
            }
        }

        println!("\nOptimizations applied: {}/{}", applied, optimizations.len());
        assert!(applied >= 8, "Should apply at least 8 optimizations");

        let compression = ((json.len() - dx.len()) as f64 / json.len() as f64) * 100.0;
        println!("Compression: {:.1}%", compression);
    }

    #[test]
    fn verify_all_formats_produce_consistent_output() {
        println!("\n=== Format Consistency Test ===");

        // Same data in different formats
        let json = r#"{"name": "app", "version": "1.0.0"}"#;
        let yaml = "name: app\nversion: 1.0.0";
        let toml = r#"name = "app"
version = "1.0.0""#;

        let dx_from_json = json_to_dx(json).expect("JSON should work");
        let dx_from_yaml = yaml_to_dx(yaml).expect("YAML should work");
        let dx_from_toml = toml_to_dx(toml).expect("TOML should work");

        println!("JSON → DX:\n{}", dx_from_json);
        println!("\nYAML → DX:\n{}", dx_from_yaml);
        println!("\nTOML → DX:\n{}", dx_from_toml);

        // All should contain optimized keys
        for dx in [&dx_from_json, &dx_from_yaml, &dx_from_toml] {
            assert!(dx.contains("n:app"), "Should have n:app");
            assert!(dx.contains("v:1.0.0"), "Should have v:1.0.0");
        }

        println!("\n✅ All formats produce consistent DX ULTRA output!");
    }
}
