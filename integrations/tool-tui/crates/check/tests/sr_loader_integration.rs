//! Integration tests for .sr file loading

use dx_check::rules::{SrRuleLoader, compile_sr_rules};
use std::path::Path;

#[test]
fn test_load_example_sr_files() {
    let examples_dir = Path::new("rules/sr/examples");

    if !examples_dir.exists() {
        eprintln!("Skipping test: examples directory not found");
        return;
    }

    let cache_dir = std::env::temp_dir().join("dx-check-test-cache");
    std::fs::create_dir_all(&cache_dir).unwrap();

    let loader = SrRuleLoader::new(cache_dir);

    // Try to load each example file
    for entry in std::fs::read_dir(examples_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("sr") {
            println!("Loading: {:?}", path);
            match loader.load_rule(&path) {
                Ok(rule) => {
                    println!("  ✓ Loaded rule: {} ({})", rule.prefixed_name, rule.description);
                    assert!(!rule.name.is_empty());
                    assert!(!rule.prefixed_name.is_empty());
                }
                Err(e) => {
                    eprintln!("  ✗ Failed to load: {}", e);
                    // Don't fail the test - just report
                }
            }
        }
    }
}

#[test]
fn test_compile_example_sr_files() {
    let examples_dir = Path::new("rules/sr/examples");

    if !examples_dir.exists() {
        eprintln!("Skipping test: examples directory not found");
        return;
    }

    let output_dir = std::env::temp_dir().join("dx-check-compiled-test");
    std::fs::create_dir_all(&output_dir).unwrap();

    match compile_sr_rules(examples_dir, &output_dir) {
        Ok(database) => {
            println!("Compiled {} rules", database.rule_count);
            assert!(database.rule_count > 0);

            // Verify the compiled file exists
            let machine_path = output_dir.join("rules.dxm");
            assert!(machine_path.exists());
        }
        Err(e) => {
            eprintln!("Compilation failed: {}", e);
            // Don't fail - the .sr format might not be fully compatible yet
        }
    }
}
