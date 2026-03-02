//! Test academic, video, music, and image engines
//! Run with: cargo test -p metasearch-engine --test test_academic_video_music_images -- --nocapture

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
        None => return (false, 0, format!("❌ ENGINE NOT FOUND in registry")),
    };
    
    let query = SearchQuery::new(query);
    
    match tokio::time::timeout(Duration::from_secs(12), engine.search(&query)).await {
        Ok(Ok(results)) => {
            let count = results.len();
            if count > 0 {
                (true, count, format!("✅ {} results", count))
            } else {
                (false, 0, "⚠️  0 results".to_string())
            }
        }
        Ok(Err(e)) => {
            let err_msg = format!("{}", e);
            if err_msg.contains("API key") || err_msg.contains("api_key") {
                (false, 0, format!("⚙️  NEEDS CONFIG: {}", err_msg))
            } else {
                (false, 0, format!("❌ ERROR: {}", err_msg))
            }
        }
        Err(_) => (false, 0, "❌ TIMEOUT".to_string()),
    }
}

#[tokio::test]
async fn test_academic_engines() {
    println!("\n{}", "=".repeat(80));
    println!("TESTING ACADEMIC & RESEARCH ENGINES (15)");
    println!("{}", "=".repeat(80));
    
    let engines = vec![
        ("arxiv", "machine learning"),
        ("crossref", "neural networks"),
        ("semantic_scholar", "deep learning"),
        ("google_scholar", "artificial intelligence"),
        ("openalex", "quantum computing"),
        ("pubmed", "cancer research"),
        ("springer", "computer science"),
        ("core", "data mining"),
        ("ads", "black holes"),
        ("astrophysics_data_system", "dark matter"),
        ("base_search", "climate change"),
        ("pdbe", "protein structure"),
        ("scanr_structures", "research"),
        ("mrs", "medical"),
        ("nvd", "CVE-2023"),
    ];
    
    let mut working = 0;
    let mut empty = 0;
    let mut errors = 0;
    let mut needs_config = 0;
    
    for (name, query) in engines {
        let (success, count, msg) = test_engine(name, query).await;
        println!("{:<30} {}", name, msg);
        
        if success {
            working += 1;
        } else if msg.contains("NEEDS CONFIG") {
            needs_config += 1;
        } else if msg.contains("0 results") {
            empty += 1;
        } else {
            errors += 1;
        }
    }
    
    println!("\n{}", "=".repeat(80));
    println!("ACADEMIC SUMMARY:");
    println!("  ✅ Working:      {}/15", working);
    println!("  ⚙️  Need Config:  {}/15", needs_config);
    println!("  ⚠️  Empty:        {}/15", empty);
    println!("  ❌ Errors:       {}/15", errors);
    println!("{}", "=".repeat(80));
}

#[tokio::test]
async fn test_video_platforms() {
    println!("\n{}", "=".repeat(80));
    println!("TESTING VIDEO PLATFORMS (15)");
    println!("{}", "=".repeat(80));
    
    let engines = vec![
        ("youtube", "cat videos"),
        ("youtube_api", "music"),
        ("youtube_noapi", "tutorial"),
        ("Invidious", "documentary"),
        ("piped", "news"),
        ("vimeo", "short film"),
        ("dailymotion", "sports"),
        ("peertube", "open source"),
        ("sepiasearch", "education"),
        ("rumble", "podcast"),
        ("odysee", "technology"),
        ("bitchute", "interview"),
        ("bilibili", "anime"),
        ("niconico", "music"),
        ("iqiyi", "drama"),
    ];
    
    let mut working = 0;
    let mut empty = 0;
    let mut errors = 0;
    let mut needs_config = 0;
    
    for (name, query) in engines {
        let (success, _count, msg) = test_engine(name, query).await;
        println!("{:<30} {}", name, msg);
        
        if success {
            working += 1;
        } else if msg.contains("NEEDS CONFIG") {
            needs_config += 1;
        } else if msg.contains("0 results") {
            empty += 1;
        } else {
            errors += 1;
        }
    }
    
    println!("\n{}", "=".repeat(80));
    println!("VIDEO SUMMARY:");
    println!("  ✅ Working:      {}/15", working);
    println!("  ⚙️  Need Config:  {}/15", needs_config);
    println!("  ⚠️  Empty:        {}/15", empty);
    println!("  ❌ Errors:       {}/15", errors);
    println!("{}", "=".repeat(80));
}

#[tokio::test]
async fn test_music_audio() {
    println!("\n{}", "=".repeat(80));
    println!("TESTING MUSIC & AUDIO ENGINES (10)");
    println!("{}", "=".repeat(80));
    
    let engines = vec![
        ("spotify", "rock music"),
        ("soundcloud", "electronic"),
        ("bandcamp", "indie"),
        ("deezer", "pop"),
        ("mixcloud", "dj mix"),
        ("freesound", "drum"),
        ("genius", "lyrics"),
        ("yandex_music", "classical"),
        ("fyyd", "podcast"),
        ("podcastindex", "technology"),
    ];
    
    let mut working = 0;
    let mut empty = 0;
    let mut errors = 0;
    let mut needs_config = 0;
    
    for (name, query) in engines {
        let (success, _count, msg) = test_engine(name, query).await;
        println!("{:<30} {}", name, msg);
        
        if success {
            working += 1;
        } else if msg.contains("NEEDS CONFIG") {
            needs_config += 1;
        } else if msg.contains("0 results") {
            empty += 1;
        } else {
            errors += 1;
        }
    }
    
    println!("\n{}", "=".repeat(80));
    println!("MUSIC SUMMARY:");
    println!("  ✅ Working:      {}/10", working);
    println!("  ⚙️  Need Config:  {}/10", needs_config);
    println!("  ⚠️  Empty:        {}/10", empty);
    println!("  ❌ Errors:       {}/10", errors);
    println!("{}", "=".repeat(80));
}

#[tokio::test]
async fn test_images_art() {
    println!("\n{}", "=".repeat(80));
    println!("TESTING IMAGES & ART ENGINES (20)");
    println!("{}", "=".repeat(80));
    
    let engines = vec![
        ("flickr", "landscape"),
        ("flickr_noapi", "portrait"),
        ("unsplash", "nature"),
        ("pixabay", "sunset"),
        ("pexels", "city"),
        ("imgur", "meme"),
        ("deviantart", "digital art"),
        ("artstation", "concept art"),
        ("pinterest", "design"),
        ("openverse", "creative commons"),
        ("wikicommons", "historical"),
        ("artic", "painting"),
        ("adobe_stock", "business"),
        ("ipernity", "photography"),
        ("www1x", "fine art"),
        ("wallhaven", "wallpaper"),
        ("openclipart", "vector"),
        ("uxwing", "icon"),
        ("pixiv", "illustration"),
        ("public_domain_image_archive", "vintage"),
    ];
    
    let mut working = 0;
    let mut empty = 0;
    let mut errors = 0;
    let mut needs_config = 0;
    
    for (name, query) in engines {
        let (success, _count, msg) = test_engine(name, query).await;
        println!("{:<30} {}", name, msg);
        
        if success {
            working += 1;
        } else if msg.contains("NEEDS CONFIG") {
            needs_config += 1;
        } else if msg.contains("0 results") {
            empty += 1;
        } else {
            errors += 1;
        }
    }
    
    println!("\n{}", "=".repeat(80));
    println!("IMAGES SUMMARY:");
    println!("  ✅ Working:      {}/20", working);
    println!("  ⚙️  Need Config:  {}/20", needs_config);
    println!("  ⚠️  Empty:        {}/20", empty);
    println!("  ❌ Errors:       {}/20", errors);
    println!("{}", "=".repeat(80));
}

#[tokio::test]
async fn test_all_60_engines() {
    println!("\n{}", "=".repeat(80));
    println!("TESTING ALL 60 ENGINES (Academic + Video + Music + Images)");
    println!("{}", "=".repeat(80));
    
    // Note: Run individual tests separately
    println!("Run individual tests:");
    println!("  cargo test test_academic_engines");
    println!("  cargo test test_video_platforms");
    println!("  cargo test test_music_audio");
    println!("  cargo test test_images_art");
}
