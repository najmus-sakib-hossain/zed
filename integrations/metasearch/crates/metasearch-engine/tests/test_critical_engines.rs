//! Test critical general web search engines
//! Run with: cargo test -p metasearch-engine --test test_critical_engines -- --nocapture

use metasearch_core::query::SearchQuery;
use metasearch_engine::registry::EngineRegistry;
use reqwest::Client;
use std::time::Duration;

async fn test_engine(name: &str, query: &str) -> (bool, usize, String) {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .expect("Failed to create HTTP client");

    let registry = EngineRegistry::with_defaults(client);
    let engine = match registry.get(name) {
        Some(e) => e,
        None => return (false, 0, format!("Engine '{}' not found in registry", name)),
    };
    
    let query = SearchQuery::new(query);
    
    match tokio::time::timeout(Duration::from_secs(12), engine.search(&query)).await {
        Ok(Ok(results)) => {
            let count = results.len();
            if count > 0 {
                (true, count, format!("✅ SUCCESS: {} results", count))
            } else {
                (false, 0, "⚠️  EMPTY: 0 results (may need different query or has bot protection)".to_string())
            }
        }
        Ok(Err(e)) => (false, 0, format!("❌ ERROR: {}", e)),
        Err(_) => (false, 0, "❌ TIMEOUT: Request took too long".to_string()),
    }
}

#[tokio::test]
async fn test_general_web_search() {
    println!("\n{}", "=".repeat(80));
    println!("TESTING GENERAL WEB SEARCH ENGINES");
    println!("{}", "=".repeat(80));
    
    let engines = vec![
        ("google", "rust programming"),
        ("duckduckgo", "rust programming"),
        ("duckduckgo_extra", "rust programming"),
        ("brave", "rust programming"),
        ("bing", "rust programming"),
        ("yahoo", "rust programming"),
        ("qwant", "rust programming"),
        ("mojeek", "rust programming"),
        ("yandex", "rust programming"),
        ("startpage", "rust programming"),
    ];
    
    let mut working = 0;
    let mut empty = 0;
    let mut errors = 0;
    
    for (name, query) in engines {
        let (success, count, msg) = test_engine(name, query).await;
        println!("\n{:<25} {}", name, msg);
        
        if success {
            working += 1;
        } else if msg.contains("EMPTY") {
            empty += 1;
        } else {
            errors += 1;
        }
    }
    
    println!("\n{}", "=".repeat(80));
    println!("SUMMARY:");
    println!("  ✅ Working: {}/10", working);
    println!("  ⚠️  Empty:   {}/10", empty);
    println!("  ❌ Errors:  {}/10", errors);
    println!("{}", "=".repeat(80));
}

#[tokio::test]
async fn test_search_engine_variants() {
    println!("\n{}", "=".repeat(80));
    println!("TESTING SEARCH ENGINE VARIANTS");
    println!("{}", "=".repeat(80));
    
    let engines = vec![
        ("bing_images", "cat"),
        ("bing_news", "technology"),
        ("bing_videos", "cat"),
        ("google_images", "cat"),
        ("Google News", "technology"),  // Note: capital G and space
        ("Google Videos", "cat"),  // Note: capital G and space
        ("google_scholar", "machine learning"),
        ("sogou", "rust"),
        ("sogou_images", "cat"),
        ("sogou_videos", "cat"),
        ("sogou_wechat", "technology"),
        ("360search", "rust"),  // Note: different name
        ("360search_videos", "cat"),  // Note: different name
        ("baidu", "rust"),
        ("chinaso", "rust"),
    ];
    
    let mut working = 0;
    let mut empty = 0;
    let mut errors = 0;
    
    for (name, query) in engines {
        let (success, _count, msg) = test_engine(name, query).await;
        println!("\n{:<30} {}", name, msg);
        
        if success {
            working += 1;
        } else if msg.contains("EMPTY") {
            empty += 1;
        } else {
            errors += 1;
        }
    }
    
    println!("\n{}", "=".repeat(80));
    println!("SUMMARY:");
    println!("  ✅ Working: {}/15", working);
    println!("  ⚠️  Empty:   {}/15", empty);
    println!("  ❌ Errors:  {}/15", errors);
    println!("{}", "=".repeat(80));
}
