//! Debug to see actual HTML responses

use reqwest::Client;
use std::time::Duration;

#[tokio::test]
async fn debug_duckduckgo_html() {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap();

    let body = format!("q={}", urlencoding::encode("test"));
    
    let resp = client
        .post("https://html.duckduckgo.com/html/")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64; rv:109.0) Gecko/20100101 Firefox/115.0")
        .body(body)
        .send()
        .await
        .unwrap();

    let html = resp.text().await.unwrap();
    
    // Check for key elements
    println!("HTML length: {}", html.len());
    
    if html.len() < 1000 {
        println!("\nFull response:\n{}", html);
    }
    
    println!("Contains 'web-result': {}", html.contains("web-result"));
    println!("Contains 'result__a': {}", html.contains("result__a"));
    println!("Contains 'result__snippet': {}", html.contains("result__snippet"));
    
    // Print first result div if exists
    if let Some(start) = html.find("web-result") {
        let snippet = &html[start..std::cmp::min(start + 500, html.len())];
        println!("\nFirst web-result snippet:\n{}", snippet);
    }
}

#[tokio::test]
async fn debug_google_html() {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap();

    let url = format!(
        "https://www.google.com/search?q={}&start=0&num=10&hl=en&lr=lang_en&ie=utf8&oe=utf8&filter=0",
        urlencoding::encode("test")
    );
    
    let resp = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0")
        .header("Accept", "*/*")
        .header("Cookie", "CONSENT=YES+")
        .send()
        .await
        .unwrap();

    let html = resp.text().await.unwrap();
    
    println!("HTML length: {}", html.len());
    println!("Contains '<h3': {}", html.contains("<h3"));
    println!("Contains '<a href': {}", html.contains("<a href"));
    println!("Contains 'class=\"r\"': {}", html.contains("class=\"r\""));
    println!("Contains 'class=\"s\"': {}", html.contains("class=\"s\""));
    println!("Contains 'class=\"st\"': {}", html.contains("class=\"st\""));
    
    // Print first few <a href links
    let mut count = 0;
    for (i, _) in html.match_indices("<a href=\"http") {
        if count < 3 {
            let snippet = &html[i..std::cmp::min(i + 200, html.len())];
            println!("\nLink {}: {}", count + 1, snippet);
            count += 1;
        }
    }
}

#[tokio::test]
async fn debug_1337x_html() {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap();

    let url = format!("https://1337x.to/search/{}/1/", urlencoding::encode("test"));
    
    let resp = client.get(&url).send().await.unwrap();
    let html = resp.text().await.unwrap();
    
    println!("HTML length: {}", html.len());
    println!("Contains 'table-list': {}", html.contains("table-list"));
    println!("Contains 'tbody': {}", html.contains("tbody"));
    println!("Contains 'table': {}", html.contains("<table"));
    println!("Contains 'Cloudflare': {}", html.contains("Cloudflare"));
    println!("Contains 'captcha': {}", html.contains("captcha"));
    
    // Print first 1000 chars
    println!("\nFirst 1000 chars:\n{}", &html[..std::cmp::min(1000, html.len())]);
}
