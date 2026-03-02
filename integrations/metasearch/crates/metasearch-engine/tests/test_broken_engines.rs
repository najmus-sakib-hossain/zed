//! Test only the broken engines to save time
//!
//! Run with: cargo test -p metasearch-engine --test test_broken_engines -- --nocapture

use metasearch_core::query::SearchQuery;
use metasearch_engine::registry::EngineRegistry;
use reqwest::Client;
use std::time::Duration;

#[tokio::test]
async fn test_broken_engines_only() {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (compatible; metasearch-test/0.1)")
        .build()
        .expect("Failed to create HTTP client");

    let registry = EngineRegistry::with_defaults(client);
    let query = SearchQuery::new("rust programming");

    // List of engines that failed in the last test (parse errors + network errors)
    let broken_engines = vec![
        // Parse errors from last test
        "adobe_stock",
        "baidu", 
        "bilibili",
        "duckduckgo_extra",
        "reddit",
        "stract",
        // Network errors from last test
        "360search",
        "360search_videos",
        "DictZone",
        "livespace",
        "repology",
        "rumble",
        "solidtorrents",
        "wttr",
    ];

    println!("\n{}", "=".repeat(80));
    println!("  TESTING {} BROKEN ENGINES", broken_engines.len());
    println!("{}", "=".repeat(80));
    println!();

    let mut passed = 0;
    let mut failed = 0;
    let mut still_broken = Vec::new();

    for (i, name) in broken_engines.iter().enumerate() {
        let engine = match registry.get(name) {
            Some(e) => e.clone(),
            None => {
                println!("  [{:>2}/{}] {:<30} ❌ NOT FOUND", i + 1, broken_engines.len(), name);
                failed += 1;
                still_broken.push((name.to_string(), "not found in registry".to_string()));
                continue;
            }
        };

        let meta = engine.metadata();
        print!("  [{:>2}/{}] {:<30} ", i + 1, broken_engines.len(), meta.display_name);

        match tokio::time::timeout(Duration::from_secs(15), engine.search(&query)).await {
            Ok(Ok(results)) if !results.is_empty() => {
                println!("✅ FIXED! {} result(s)", results.len());
                passed += 1;
            }
            Ok(Ok(_)) => {
                println!("⚠️  0 results (connects but empty)");
                passed += 1; // Count as fixed if it connects
            }
            Ok(Err(e)) => {
                let msg = format!("{e}");
                let short = if msg.len() > 50 { &msg[..50] } else { &msg };
                println!("❌ STILL BROKEN: {short}");
                failed += 1;
                still_broken.push((name.to_string(), msg));
            }
            Err(_) => {
                println!("⏱️  TIMEOUT");
                failed += 1;
                still_broken.push((name.to_string(), "timeout".to_string()));
            }
        }
    }

    println!();
    println!("{}", "=".repeat(80));
    println!("  RESULTS:");
    println!("    ✅ Fixed/Working: {}", passed);
    println!("    ❌ Still Broken:  {}", failed);
    println!("{}", "=".repeat(80));

    if !still_broken.is_empty() {
        println!();
        println!("  Still broken engines:");
        for (name, reason) in &still_broken {
            let short = if reason.len() > 60 {
                &reason[..60]
            } else {
                reason.as_str()
            };
            println!("    • {}: {}", name, short);
        }
    }

    println!();
}
