//! Debug single engine to see actual output

use metasearch_core::query::SearchQuery;
use metasearch_engine::registry::EngineRegistry;
use reqwest::Client;
use std::time::Duration;

#[tokio::test]
async fn debug_duckduckgo() {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (compatible; metasearch-test/0.1)")
        .build()
        .expect("Failed to create HTTP client");

    let registry = EngineRegistry::with_defaults(client);
    let engine = registry.get("duckduckgo").unwrap();
    let query = SearchQuery::new("test");
    
    match engine.search(&query).await {
        Ok(results) => {
            println!("✅ DuckDuckGo: {} results", results.len());
            for (i, r) in results.iter().take(5).enumerate() {
                println!("  {}. {} - {}", i+1, r.title, r.url);
            }
        }
        Err(e) => {
            println!("❌ DuckDuckGo Error: {}", e);
        }
    }
}

#[tokio::test]
async fn debug_google() {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (compatible; metasearch-test/0.1)")
        .build()
        .expect("Failed to create HTTP client");

    let registry = EngineRegistry::with_defaults(client);
    let engine = registry.get("google").unwrap();
    let query = SearchQuery::new("test");
    
    match engine.search(&query).await {
        Ok(results) => {
            println!("✅ Google: {} results", results.len());
            for (i, r) in results.iter().take(5).enumerate() {
                println!("  {}. {} - {}", i+1, r.title, r.url);
            }
        }
        Err(e) => {
            println!("❌ Google Error: {}", e);
        }
    }
}

#[tokio::test]
async fn debug_1337x() {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (compatible; metasearch-test/0.1)")
        .build()
        .expect("Failed to create HTTP client");

    let registry = EngineRegistry::with_defaults(client);
    let engine = registry.get("1337x").unwrap();
    let query = SearchQuery::new("test");
    
    match engine.search(&query).await {
        Ok(results) => {
            println!("✅ 1337x: {} results", results.len());
            for (i, r) in results.iter().take(5).enumerate() {
                println!("  {}. {} - {}", i+1, r.title, r.url);
            }
        }
        Err(e) => {
            println!("❌ 1337x Error: {}", e);
        }
    }
}
