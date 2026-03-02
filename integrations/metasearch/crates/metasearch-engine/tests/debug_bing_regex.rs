//! Test regex extraction

use reqwest::Client;
use std::time::Duration;

#[tokio::test]
async fn test_bing_regex() {
    let client = Client::builder().timeout(Duration::from_secs(15)).build().unwrap();
    let url = "https://www.bing.com/videos/asyncv2?q=cat&async=content&first=1&count=35";
    let resp = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .unwrap();
    let html = resp.text().await.unwrap();
    
    // Test different regex patterns
    let pattern1 = regex::Regex::new(r#"mmeta=\{&quot;([^}]+)\}&quot;"#).unwrap();
    let pattern2 = regex::Regex::new(r#"mmeta="?\{&quot;([^}]+)\}&quot;"?"#).unwrap();
    let pattern3 = regex::Regex::new(r#"mmeta=\{&quot;mid&quot;:&quot;([^&]+)&quot;"#).unwrap();
    
    println!("Pattern 1 matches: {}", pattern1.captures_iter(&html).count());
    println!("Pattern 2 matches: {}", pattern2.captures_iter(&html).count());
    println!("Pattern 3 matches: {}", pattern3.captures_iter(&html).count());
    
    // Show first match
    if let Some(cap) = pattern1.captures(&html) {
        println!("\nFirst match (pattern 1):");
        println!("{:?}", &cap[0][..std::cmp::min(200, cap[0].len())]);
    }
    
    // Try simpler pattern
    let simple = regex::Regex::new(r#"mmeta="#).unwrap();
    println!("\nSimple 'mmeta=' count: {}", simple.find_iter(&html).count());
    
    // Show actual mmeta in HTML
    if let Some(pos) = html.find("mmeta=") {
        println!("\nActual mmeta in HTML:");
        println!("{}", &html[pos..std::cmp::min(pos + 300, html.len())]);
    }
}
