//! Debug image engines to see actual image URLs
//! Run with: cargo test -p metasearch-engine --test debug_image_engines -- --nocapture

use metasearch_core::query::SearchQuery;
use metasearch_engine::registry::EngineRegistry;
use reqwest::Client;
use std::time::Duration;

async fn debug_image_engine(name: &str, query: &str) {
    println!("\n{}", "=".repeat(80));
    println!("🔍 DEBUGGING: {}", name);
    println!("{}", "=".repeat(80));
    
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .expect("Failed to create HTTP client");

    let registry = EngineRegistry::with_defaults(client);
    let engine = match registry.get(name) {
        Some(e) => e,
        None => {
            println!("❌ Engine '{}' not found in registry", name);
            return;
        }
    };
    
    let query = SearchQuery::new(query);
    
    match tokio::time::timeout(Duration::from_secs(12), engine.search(&query)).await {
        Ok(Ok(results)) => {
            println!("✅ Got {} results\n", results.len());
            
            for (i, result) in results.iter().take(5).enumerate() {
                println!("Result #{}:", i + 1);
                println!("  Title: {}", result.title);
                println!("  URL: {}", result.url);
                println!("  Thumbnail: {:?}", result.thumbnail);
                println!("  Metadata: {}", result.metadata);
                println!();
            }
        }
        Ok(Err(e)) => {
            println!("❌ Error: {}", e);
        }
        Err(_) => {
            println!("❌ Timeout");
        }
    }
}

#[tokio::test]
async fn debug_google_images() {
    debug_image_engine("google_images", "cat").await;
}

#[tokio::test]
async fn debug_bing_images() {
    debug_image_engine("bing_images", "cat").await;
}

#[tokio::test]
async fn debug_flickr() {
    debug_image_engine("flickr", "landscape").await;
}

#[tokio::test]
async fn debug_unsplash() {
    debug_image_engine("unsplash", "nature").await;
}

#[tokio::test]
async fn debug_pixabay() {
    debug_image_engine("pixabay", "sunset").await;
}

#[tokio::test]
async fn debug_imgur() {
    debug_image_engine("imgur", "meme").await;
}

#[tokio::test]
async fn debug_pinterest() {
    debug_image_engine("pinterest", "design").await;
}

#[tokio::test]
async fn debug_all_image_engines() {
    let engines = vec![
        ("google_images", "cat"),
        ("bing_images", "cat"),
        ("flickr", "landscape"),
        ("unsplash", "nature"),
        ("pixabay", "sunset"),
        ("imgur", "meme"),
        ("pinterest", "design"),
        ("artstation", "concept art"),
        ("deviantart", "digital art"),
    ];
    
    for (name, query) in engines {
        debug_image_engine(name, query).await;
    }
}
