//! Detailed Bing Videos debug

use reqwest::Client;
use scraper::{Html, Selector};
use std::time::Duration;

#[tokio::test]
async fn debug_bing_videos_parsing() {
    let client = Client::builder().timeout(Duration::from_secs(15)).build().unwrap();
    let url = "https://www.bing.com/videos/asyncv2?q=cat&async=content&first=1&count=35";
    let resp = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await
        .unwrap();
    let html = resp.text().await.unwrap();
    
    // Check raw HTML structure
    if let Some(pos) = html.find("mc_vtvc_video") {
        let snippet = &html[pos.saturating_sub(100)..std::cmp::min(pos + 400, html.len())];
        println!("\nRaw HTML around mc_vtvc_video:\n{}", snippet);
    }
    
    let document = Html::parse_document(&html);
    
    // Try different selectors
    let video_sel1 = Selector::parse("div[id^='mc_vtvc_video_']").unwrap();
    let video_sel2 = Selector::parse("div.mc_vtvc").unwrap();
    let video_sel3 = Selector::parse("div[class*='mc_vtvc']").unwrap();
    let video_sel4 = Selector::parse("div[mmeta]").unwrap();
    let video_sel5 = Selector::parse("div").unwrap();
    
    println!("\nSelector 'div[id^=mc_vtvc_video_]': {} matches", document.select(&video_sel1).count());
    println!("Selector 'div.mc_vtvc': {} matches", document.select(&video_sel2).count());
    println!("Selector 'div[class*=mc_vtvc]': {} matches", document.select(&video_sel3).count());
    println!("Selector 'div[mmeta]': {} matches", document.select(&video_sel4).count());
    
    // Count all divs and check for mmeta attribute manually
    let mut divs_with_mmeta = 0;
    for div in document.select(&video_sel5) {
        if div.value().attr("mmeta").is_some() {
            divs_with_mmeta += 1;
            if divs_with_mmeta <= 2 {
                println!("\nFound div with mmeta!");
                println!("  id: {:?}", div.value().attr("id"));
                println!("  class: {:?}", div.value().attr("class"));
            }
        }
    }
    println!("\nTotal divs with mmeta attribute: {}", divs_with_mmeta);
}
