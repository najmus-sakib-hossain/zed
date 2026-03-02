//! Debug Google response to see what we're getting
//! Run with: cargo test -p metasearch-engine --test debug_google_response -- --nocapture

use reqwest::Client;
use std::time::Duration;

#[tokio::test]
async fn debug_google_response() {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .expect("Failed to create HTTP client");

    let query = "rust programming";
    
    // Generate arc_id similar to SearXNG
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let arc_id = format!("arc_id:srp_{}_{:02}", timestamp / 3600, 0);
    let async_param = format!("{},use_ac:true,_fmt:prog", arc_id);

    let url = format!(
        "https://www.google.com/search?q={}&start=0&hl=en&lr=lang_en&ie=utf8&oe=utf8&filter=0&safe=off&asearch=arc&async={}",
        urlencoding::encode(query),
        urlencoding::encode(&async_param),
    );

    println!("\n🔍 Testing Google with URL:");
    println!("{}\n", url);

    let resp = client
        .get(&url)
        .header("Accept", "*/*")
        .header("Accept-Language", "en,en-US;q=0.9")
        .header("Accept-Encoding", "gzip, deflate, br")
        .header("Referer", "https://www.google.com/")
        .header("Cookie", "CONSENT=YES+")
        .send()
        .await
        .expect("Failed to send request");

    println!("📊 Response Status: {}", resp.status());
    println!("📍 Final URL: {}", resp.url());
    
    let headers = resp.headers().clone();
    println!("\n📋 Response Headers:");
    for (name, value) in headers.iter() {
        println!("  {}: {:?}", name, value);
    }

    let body = resp.text().await.expect("Failed to get body");
    
    println!("\n📄 Response Length: {} bytes", body.len());
    println!("\n📝 First 2000 characters:");
    println!("{}", &body[..body.len().min(2000)]);
    
    // Check for common bot detection patterns
    println!("\n🤖 Bot Detection Check:");
    if body.contains("sorry") || body.contains("captcha") || body.contains("unusual traffic") {
        println!("  ⚠️  DETECTED: Bot protection active!");
    } else {
        println!("  ✅ No obvious bot protection");
    }
    
    // Check for result containers
    println!("\n🔍 Looking for result containers:");
    let containers = vec!["MjjYud", "g\"", "tF2Cxc", "data-sokoban"];
    for container in containers {
        let count = body.matches(container).count();
        println!("  {} found: {} times", container, count);
    }
}
