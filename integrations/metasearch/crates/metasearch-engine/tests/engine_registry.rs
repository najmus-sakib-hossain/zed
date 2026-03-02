//! Smoke test: verify every engine registers and metadata is sane.

use metasearch_engine::EngineRegistry;
use reqwest::Client;

#[test]
fn test_all_engines_register() {
    let client = Client::new();
    let registry = EngineRegistry::with_defaults(client);

    // We expect at least 70 engines (65 existing + 5 new in Batch 8)
    let count = registry.count();
    assert!(count >= 70, "Expected at least 70 engines, found {count}",);

    // Every engine must have non-empty name and display_name
    for name in registry.engine_names() {
        let engine = registry.get(&name).expect("engine must exist");
        let meta = engine.metadata();
        assert!(!meta.name.is_empty(), "Engine name must not be empty");
        assert!(
            !meta.display_name.is_empty(),
            "Engine display_name must not be empty for {}",
            meta.name
        );
        assert!(
            !meta.homepage.is_empty(),
            "Engine homepage must not be empty for {}",
            meta.name
        );
        assert!(
            !meta.categories.is_empty(),
            "Engine must have at least one category: {}",
            meta.name
        );
        assert!(
            meta.timeout_ms > 0,
            "Engine timeout must be > 0: {}",
            meta.name
        );
        assert!(
            meta.weight > 0.0,
            "Engine weight must be > 0: {}",
            meta.name
        );
    }

    println!("✅ All {count} engines registered with valid metadata");
}

#[tokio::test]
async fn test_engines_can_instantiate_and_have_unique_names() {
    let client = Client::new();
    let registry = EngineRegistry::with_defaults(client);

    let names = registry.engine_names();
    let mut seen = std::collections::HashSet::new();
    for name in &names {
        assert!(seen.insert(name.clone()), "Duplicate engine name: {name}",);
    }

    println!("✅ All {} engine names are unique", names.len());
}
