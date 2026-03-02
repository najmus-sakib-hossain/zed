//! Debug specific empty engines to see what they're returning
//!
//! Run with: cargo test -p metasearch-engine --test debug_empty_specific -- --nocapture

use metasearch_core::query::SearchQuery;
use metasearch_engine::registry::EngineRegistry;
use reqwest::Client;
use std::time::Duration;

#[tokio::test]
async fn debug_deviantart() {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0")
        .build()
        .unwrap();

    let registry = EngineRegistry::with_defaults(client.clone());
    let engine = registry.get("deviantart").unwrap();
    
    let query = SearchQuery::new("cat");
    
    println!("\n=== Testing DeviantArt with query 'cat' ===");
    
    // Get raw HTML
    let url = format!("https://www.deviantart.com/search?q={}", urlencoding::encode("cat"));
    let resp = client.get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0")
        .send()
        .await
        .unwrap();
    
    let html = resp.text().await.unwrap();
    println!("HTML length: {}", html.len());
    println!("Contains 'deviation_link': {}", html.contains("deviation_link"));
    println!("Contains 'data-hook': {}", html.contains("data-hook"));
    
    // Save to file
    std::fs::write("debug_deviantart.html", &html).unwrap();
    println!("Saved to debug_deviantart.html");
    
    // Try engine
    match engine.search(&query).await {
        Ok(results) => println!("Results: {}", results.len()),
        Err(e) => println!("Error: {}", e),
    }
}

#[tokio::test]
async fn debug_searchcode() {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap();

    let registry = EngineRegistry::with_defaults(client.clone());
    let engine = registry.get("searchcode_code").unwrap();
    
    let query = SearchQuery::new("test");
    
    println!("\n=== Testing Searchcode with query 'test' ===");
    
    // Get raw response
    let url = "https://searchcode.com/api/codesearch_I/?q=test";
    let resp = client.get(url).send().await.unwrap();
    
    let text = resp.text().await.unwrap();
    println!("Response length: {}", text.len());
    println!("First 500 chars: {}", &text[..text.len().min(500)]);
    
    // Try engine
    match engine.search(&query).await {
        Ok(results) => println!("Results: {}", results.len()),
        Err(e) => println!("Error: {}", e),
    }
}

#[tokio::test]
async fn debug_sourcehut() {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0")
        .build()
        .unwrap();

    let registry = EngineRegistry::with_defaults(client.clone());
    let engine = registry.get("sourcehut").unwrap();
    
    let query = SearchQuery::new("test");
    
    println!("\n=== Testing SourceHut with query 'test' ===");
    
    // Get raw HTML
    let url = "https://sr.ht/projects?search=test&page=1&sort=recently-updated";
    let resp = client.get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0")
        .send()
        .await
        .unwrap();
    
    let html = resp.text().await.unwrap();
    println!("HTML length: {}", html.len());
    println!("Contains 'event-list': {}", html.contains("event-list"));
    println!("Contains 'event': {}", html.contains("event"));
    
    // Save to file
    std::fs::write("debug_sourcehut.html", &html).unwrap();
    println!("Saved to debug_sourcehut.html");
    
    // Try engine
    match engine.search(&query).await {
        Ok(results) => println!("Results: {}", results.len()),
        Err(e) => println!("Error: {}", e),
    }
}

#[tokio::test]
async fn debug_flickr() {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0")
        .build()
        .unwrap();

    let registry = EngineRegistry::with_defaults(client.clone());
    let engine = registry.get("flickr_noapi").unwrap();
    
    let query = SearchQuery::new("cat");
    
    println!("\n=== Testing Flickr with query 'cat' ===");
    
    // Get raw HTML
    let url = "https://www.flickr.com/search/?text=cat&page=1";
    let resp = client.get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0")
        .send()
        .await
        .unwrap();
    
    let html = resp.text().await.unwrap();
    println!("HTML length: {}", html.len());
    println!("Contains '/photos/': {}", html.contains("/photos/"));
    
    // Save to file
    std::fs::write("debug_flickr.html", &html).unwrap();
    println!("Saved to debug_flickr.html");
    
    // Try engine
    match engine.search(&query).await {
        Ok(results) => println!("Results: {}", results.len()),
        Err(e) => println!("Error: {}", e),
    }
}
