//! Debug specific engines to see actual output

use metasearch_core::query::SearchQuery;
use metasearch_engine::registry::EngineRegistry;
use reqwest::Client;
use std::time::Duration;

async fn test_engine(name: &str, query: &str) {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (compatible; metasearch-test/0.1)")
        .build()
        .expect("Failed to create HTTP client");

    let registry = EngineRegistry::with_defaults(client);
    let engine = registry.get(name).unwrap();
    let query = SearchQuery::new(query);
    
    match engine.search(&query).await {
        Ok(results) => {
            println!("✅ {}: {} results", name, results.len());
            for (i, r) in results.iter().take(3).enumerate() {
                println!("  {}. {} - {}", i+1, r.title, r.url);
            }
        }
        Err(e) => {
            println!("❌ {} Error: {}", name, e);
        }
    }
}

#[tokio::test]
async fn debug_pypi() {
    test_engine("pypi", "requests").await;
}

#[tokio::test]
async fn debug_ebay() {
    test_engine("ebay", "laptop").await;
}

#[tokio::test]
async fn debug_bing_videos() {
    test_engine("bing_videos", "cat").await;
}

#[tokio::test]
async fn debug_github_code() {
    test_engine("github_code", "rust async").await;
}
