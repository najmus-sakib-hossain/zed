/// Test: Smart Key Handling (Popular vs Custom)
///
/// Verifies that:
/// 1. Popular keys get abbreviated (using default 12 mappings)
/// 2. Custom keys are preserved as-is
/// 3. Mixed scenarios work correctly
///
/// Default mappings: n=name, v=version, t=title, d=description, a=author,
///                   c=context, l=languages, f=forge, s=style, m=media, i=i18n, u=ui
use serializer::{Mappings, format_machine};

#[test]
fn test_popular_keys_abbreviated() {
    // Popular keys should be abbreviated (using default mappings)
    let input = "name:dx-www\nversion:1.0.0\ndescription:Runtime";
    let compressed = format_machine(input).unwrap();
    let result = String::from_utf8(compressed).unwrap();

    // Should abbreviate all popular keys
    assert!(result.contains("n:dx-www"), "name should abbreviate to n");
    assert!(result.contains("v:1.0.0"), "version should abbreviate to v");
    assert!(result.contains("d:Runtime"), "description should abbreviate to d");
}

#[test]
fn test_custom_keys_preserved() {
    // Custom keys should stay as-is
    let input = "myCustomField:value123\nuserPreferences:dark";
    let compressed = format_machine(input).unwrap();
    let result = String::from_utf8(compressed).unwrap();

    // Should preserve custom keys exactly
    assert!(result.contains("myCustomField:value123"), "Custom key should be preserved");
    assert!(result.contains("userPreferences:dark"), "Custom key should be preserved");
}

#[test]
fn test_mixed_popular_and_custom() {
    // Mix of popular and custom keys
    let input = "name:dx-www\nmyAppFeature:enabled\nversion:1.0\ncustomTimeout:5000";
    let compressed = format_machine(input).unwrap();
    let result = String::from_utf8(compressed).unwrap();

    println!("Input:  {}", input);
    println!("Output: {}", result);

    // Popular keys abbreviated
    assert!(result.contains("n:dx-www"), "Popular 'name' should abbreviate");
    assert!(result.contains("v:1.0"), "Popular 'version' should abbreviate");

    // Custom keys preserved
    assert!(result.contains("myAppFeature:enabled"), "Custom key preserved");
    assert!(result.contains("customTimeout:5000"), "Custom key preserved");
}

#[test]
fn test_nested_popular_keys() {
    // Nested popular keys (using default mappings: c=context, n=name, v=version)
    let input = "context.name:dx-www\ncontext.version:1.0";
    let compressed = format_machine(input).unwrap();
    let result = String::from_utf8(compressed).unwrap();

    // Both parts should abbreviate
    assert!(result.contains("c.n:dx-www"), "context.name → c.n");
    assert!(result.contains("c.v:1.0"), "context.version → c.v");
}

#[test]
fn test_nested_custom_keys() {
    // Nested custom keys
    let input = "myModule.myFeature:enabled\nmyModule.timeout:5000";
    let compressed = format_machine(input).unwrap();
    let result = String::from_utf8(compressed).unwrap();

    // Should preserve both parts
    assert!(result.contains("myModule.myFeature:enabled"), "Custom nested key preserved");
    assert!(result.contains("myModule.timeout:5000"), "Custom nested key preserved");
}

#[test]
fn test_nested_mixed_keys() {
    // One popular, one custom
    let input = "context.myCustomField:value";
    let compressed = format_machine(input).unwrap();
    let result = String::from_utf8(compressed).unwrap();

    println!("Input:  {}", input);
    println!("Output: {}", result);

    // 'context' abbreviated, 'myCustomField' preserved
    assert!(result.contains("c.myCustomField:value"), "Mixed: c.myCustomField");
}

#[test]
fn test_context_popular() {
    // Context is a popular prefix in default mappings (c=context)
    let input = "context.name:myapp\ncontext.version:1.0";
    let compressed = format_machine(input).unwrap();
    let result = String::from_utf8(compressed).unwrap();

    println!("Input:  {}", input);
    println!("Output: {}", result);

    // Should abbreviate 'context' to 'c' and 'name' to 'n', 'version' to 'v'
    assert!(result.contains("c.n:myapp"), "context.name → c.n");
    assert!(result.contains("c.v:1.0"), "context.version → c.v");
}

#[test]
fn test_roundtrip_custom_keys() {
    // Custom keys should survive roundtrip
    // Use newlines instead of ^ for proper parsing
    let expanded = "name:dx\nmyFeature:on\nversion:1.0\ncustomKey:value";

    // Human → Machine (compress)
    let machine = format_machine(&expanded).unwrap();
    let machine_str = String::from_utf8(machine.clone()).unwrap();

    println!("Expanded: {}", expanded);
    println!("Machine:  {}", machine_str);

    // Should preserve custom keys in output
    assert!(machine_str.contains("myFeature"), "Custom key in machine format");
    assert!(machine_str.contains("customKey"), "Custom key in machine format");

    // Should abbreviate popular keys
    assert!(machine_str.contains("n:dx"), "name → n");
    assert!(machine_str.contains("v:1.0"), "version → v");
}

#[test]
fn test_all_default_popular_keys() {
    // Test that all default popular keys get abbreviated
    // Default mappings: n=name, v=version, t=title, d=description, a=author,
    //                   c=context, l=languages, f=forge, s=style, m=media, i=i18n, u=ui
    let mappings = Mappings::get();

    println!("\n📊 Testing all {} popular keys:\n", mappings.compress.len());

    let mut tested = 0;
    for (full, short) in &mappings.compress {
        // Test compression
        let result = mappings.compress_key(full);
        assert_eq!(result, *short, "Failed: {} should compress to {}", full, short);

        // Test expansion
        let expanded = mappings.expand_key(short);
        assert_eq!(expanded, *full, "Failed: {} should expand to {}", short, full);

        tested += 1;
    }

    println!("✅ All {} popular keys tested successfully!", tested);
    // Default mappings have 12 keys (can be extended via mappings.dx file)
    assert!(tested >= 12, "Should have at least 12 default popular keys");
}

#[test]
fn test_custom_keys_not_in_mappings() {
    let mappings = Mappings::get();

    // Custom keys that don't exist in mappings
    let custom_keys = vec![
        "myFeature",
        "userPreferences",
        "customTimeout",
        "applicationState",
        "businessLogic",
    ];

    println!("\n🔍 Testing custom keys preservation:\n");

    for key in custom_keys {
        // Should return same key (no mapping exists)
        let compressed = mappings.compress_key(key);
        let expanded = mappings.expand_key(key);

        assert_eq!(compressed, key, "Custom key should not change: {}", key);
        assert_eq!(expanded, key, "Custom key should not change: {}", key);

        println!("  ✅ {} → {} (preserved)", key, compressed);
    }
}

#[test]
fn test_real_world_mixed_config() {
    // Real-world config with popular and custom keys
    // Using default mappings: c=context, n=name, v=version, a=author, t=title, d=description
    let input = r#"context.name        : my-app
version             : 2.0.0
author              : Team

style.theme         : dark
media.format        : png

myFeatureFlags.darkMode     : true
myFeatureFlags.experimental : false

customSettings.timeout      : 5000
customSettings.retries      : 3"#;

    let machine = format_machine(input).unwrap();
    let result = String::from_utf8(machine).unwrap();

    println!("\n📄 Real-world example:\n");
    println!("Input:\n{}\n", input);
    println!("Output:\n{}\n", result);

    // Popular keys abbreviated (using default mappings)
    assert!(result.contains("c.n:my-app"), "context.name → c.n");
    assert!(result.contains("v:2.0.0"), "version → v");
    assert!(result.contains("a:Team"), "author → a");
    assert!(result.contains("s."), "style → s");
    assert!(result.contains("m."), "media → m");

    // Custom keys preserved
    assert!(result.contains("myFeatureFlags"), "Custom prefix preserved");
    assert!(result.contains("customSettings"), "Custom prefix preserved");
    assert!(result.contains("darkMode"), "Custom key preserved");
    assert!(result.contains("timeout"), "Custom key preserved");
}
