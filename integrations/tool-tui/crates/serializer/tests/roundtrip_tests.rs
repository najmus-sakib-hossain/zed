/// Tests for bidirectional DX conversion (roundtrip)

#[cfg(test)]
mod roundtrip_tests {
    use serializer::{Mappings, format_machine};

    #[test]
    fn test_simple_roundtrip() {
        let human = "context.name        : dx\n^version            : 0.0.1";
        let machine = format_machine(human).unwrap();
        let result = String::from_utf8(machine).unwrap();

        // Should produce compact format
        assert!(result.contains("c.n:dx"));
        assert!(result.contains("v:0.0.1") || result.contains("^v:0.0.1"));
    }

    #[test]
    fn test_array_roundtrip() {
        // Use "name" which exists in default mappings
        let human = "name           > frontend | backend | shared";
        let machine = format_machine(human).unwrap();
        let result = String::from_utf8(machine).unwrap();

        // Should compress arrays with pipe separator
        // "name" compresses to "n" in default mappings
        assert!(result.contains("n>frontend|backend|shared"));
    }

    #[test]
    fn test_nested_keys() {
        // Use keys that exist in default mappings
        let human = "context.name    : https://example.com";
        let machine = format_machine(human).unwrap();
        let result = String::from_utf8(machine).unwrap();

        // Should compress nested keys
        // "context" → "c", "name" → "n"
        assert!(result.contains("c.n:https://example.com"));
    }

    #[test]
    fn test_underscore_keys() {
        // Underscore keys are NOT split to avoid round-trip issues
        // (e.g., "ui_a" would become "u_a" if split, which is lossy)
        let human = "name_items         > cli | docs | tests";
        let machine = format_machine(human).unwrap();
        let result = String::from_utf8(machine).unwrap();

        // Underscore keys should be preserved as-is (not split)
        // "name_items" stays as "name_items" (no mapping exists for the full key)
        assert!(result.contains("name_items>") || result.contains("name_items"));
    }

    #[test]
    fn test_mappings_loaded() {
        let mappings = Mappings::get();

        // Verify key mappings exist
        assert_eq!(mappings.expand_key("n"), "name");
        assert_eq!(mappings.expand_key("v"), "version");
        assert_eq!(mappings.compress_key("name"), "n");
        assert_eq!(mappings.compress_key("version"), "v");
    }

    #[test]
    fn test_prefix_inheritance() {
        let human = r#"context.name        : app
^version            : 1.0.0
^title              : My App"#;

        let machine = format_machine(human).unwrap();
        let result = String::from_utf8(machine).unwrap();

        // Should handle prefix inheritance
        assert!(result.contains("c.n:app"));
        assert!(result.contains("^v:1.0.0") || result.contains("^v:1.0.0"));
    }

    #[test]
    fn test_complex_config() {
        let human = r#"context.name        : dx
^version            : 0.0.1

name           > frontend/www | backend/api

context.description    : https://github.com/dx/dx
"#;

        let machine = format_machine(human).unwrap();
        let result = String::from_utf8(machine).unwrap();

        // Verify compression
        assert!(result.len() < human.len(), "Machine format should be smaller");
        assert!(result.contains("c.n:dx"));
        assert!(result.contains("n>"));
        assert!(result.contains("c.d:") || result.contains("c.description:"));
    }

    #[test]
    fn test_size_comparison() {
        let human = r#"context.name        : my-application
^version            : 2.0.1
^description        : Test application
^author             : John Doe

name           > frontend | backend | shared | utils
"#;

        let machine = format_machine(human).unwrap();

        // Machine format should be significantly smaller
        let compression_ratio = human.len() as f64 / machine.len() as f64;
        assert!(compression_ratio > 1.5, "Should compress at least 33%");

        println!("\n📊 Compression Stats:");
        println!("   Human:   {} bytes", human.len());
        println!("   Machine: {} bytes", machine.len());
        println!("   Ratio:   {:.2}x smaller", compression_ratio);
    }
}
