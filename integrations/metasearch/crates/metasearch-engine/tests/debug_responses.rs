//! Debug actual responses

use reqwest::Client;
use std::time::Duration;

#[tokio::test]
async fn debug_pypi_response() {
    let client = Client::builder().timeout(Duration::from_secs(15)).build().unwrap();
    let url = "https://pypi.org/search/?q=requests&page=1";
    let resp = client.get(url).send().await.unwrap();
    let html = resp.text().await.unwrap();
    
    println!("PyPI HTML length: {}", html.len());
    println!("Contains 'package-snippet': {}", html.contains("package-snippet"));
    println!("Contains 'package-snippet__name': {}", html.contains("package-snippet__name"));
    println!("Contains 'search-results': {}", html.contains("search-results"));
    println!("Contains 'package__snippet': {}", html.contains("package__snippet"));
    
    // Print first 2000 chars to see structure
    println!("\nFirst 2000 chars:\n{}", &html[..std::cmp::min(2000, html.len())]);
}

#[tokio::test]
async fn debug_ebay_response() {
    let client = Client::builder().timeout(Duration::from_secs(15)).build().unwrap();
    let url = "https://www.ebay.com/sch/i.html?_nkw=laptop&_sacat=1";
    let resp = client.get(url).send().await.unwrap();
    let html = resp.text().await.unwrap();
    
    println!("eBay HTML length: {}", html.len());
    println!("Contains 's-item': {}", html.contains("s-item"));
    println!("Contains 's-item__link': {}", html.contains("s-item__link"));
    println!("Contains 's-item__title': {}", html.contains("s-item__title"));
    println!("Contains 'srp-results': {}", html.contains("srp-results"));
    
    // Find actual class names
    if let Some(start) = html.find("class=\"s-item") {
        let snippet = &html[start..std::cmp::min(start + 1000, html.len())];
        println!("\nFirst s-item with context:\n{}", snippet);
    }
}

#[tokio::test]
async fn debug_bing_videos_response() {
    let client = Client::builder().timeout(Duration::from_secs(15)).build().unwrap();
    let url = "https://www.bing.com/videos/asyncv2?q=cat&async=content&first=1&count=35";
    let resp = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .unwrap();
    let html = resp.text().await.unwrap();
    
    println!("Bing Videos HTML length: {}", html.len());
    println!("Contains 'mc_vtvc_video': {}", html.contains("mc_vtvc_video"));
    println!("Contains 'dg_u': {}", html.contains("dg_u"));
    println!("Contains 'mmeta': {}", html.contains("mmeta"));
    
    if let Some(start) = html.find("mc_vtvc_video") {
        let snippet = &html[start..std::cmp::min(start + 500, html.len())];
        println!("\nFirst mc_vtvc_video:\n{}", snippet);
    }
}

#[tokio::test]
async fn debug_github_code_response() {
    let client = Client::builder().timeout(Duration::from_secs(15)).build().unwrap();
    let url = "https://api.github.com/search/code?sort=indexed&q=rust+async&page=1";
    let resp = client
        .get(url)
        .header("Accept", "application/vnd.github.text-match+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "metasearch-engine/1.0")
        .send()
        .await
        .unwrap();
    
    let status = resp.status();
    println!("GitHub Code status: {}", status);
    
    let text = resp.text().await.unwrap();
    println!("Response length: {}", text.len());
    
    if text.len() < 1000 {
        println!("Full response:\n{}", text);
    } else {
        println!("First 500 chars:\n{}", &text[..500]);
    }
}
