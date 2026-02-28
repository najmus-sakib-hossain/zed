//! Font Search Speed Test & Demo
//!
//! This example demonstrates the high-performance search capabilities of dx-font
//! with concurrent provider fetching and timing information.
//!
//! Run with: cargo run --example search_fonts

use anyhow::Result;
use dx_font::cdn::{CdnUrlGenerator, get_popular_font_cdn_urls};
use dx_font::search::FontSearch;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔═══════════════════════════════════════════════════════════════════════╗");
    println!("║           dx-font SPEED TEST & SEARCH DEMO                            ║");
    println!("║           Access 50,000+ Commercial-Free Fonts!                       ║");
    println!("╚═══════════════════════════════════════════════════════════════════════╝\n");

    // Initialize the font search engine
    let search = FontSearch::new()?;

    // ═══════════════════════════════════════════════════════════════════════════
    // TEST 1: Provider Health Check with Timing
    // ═══════════════════════════════════════════════════════════════════════════
    println!("🏥 TEST 1: Provider Health Check (Concurrent)");
    println!("─────────────────────────────────────────────────────────────────────────\n");

    let start = Instant::now();
    let health = search.health_check_timed().await;
    let total_health_time = start.elapsed();

    println!("{:<25} {:<12} {:<15}", "Provider", "Status", "Response Time");
    println!("{}", "─".repeat(55));

    for (provider, is_healthy, duration) in &health {
        let status = if *is_healthy {
            "✅ Online"
        } else {
            "❌ Offline"
        };
        println!("{:<25} {:<12} {:>8.2?}", provider, status, duration);
    }

    println!("{}", "─".repeat(55));
    println!("⏱️  Total health check time: {:?} (concurrent)\n", total_health_time);

    // ═══════════════════════════════════════════════════════════════════════════
    // TEST 2: Font Statistics with Timing
    // ═══════════════════════════════════════════════════════════════════════════
    println!("📊 TEST 2: Font Statistics (Concurrent Fetch)");
    println!("─────────────────────────────────────────────────────────────────────────\n");

    let start = Instant::now();
    let stats = search.get_stats().await?;
    let stats_time = start.elapsed();

    println!("╔═══════════════════════════════════════════════════════════════════════╗");
    println!("║                     dx-font LIBRARY STATISTICS                        ║");
    println!("╠═══════════════════════════════════════════════════════════════════════╣");
    println!(
        "║  Total Indexed Fonts:  {:>6}                                        ║",
        stats.total_fonts
    );
    println!(
        "║  Active Providers:     {:>6}                                        ║",
        stats.providers_count
    );
    println!(
        "║  Fetch Time:           {:>6} ms                                     ║",
        stats.fetch_time_ms
    );
    println!("╠═══════════════════════════════════════════════════════════════════════╣");
    println!(
        "║  Serif:                {:>6}                                        ║",
        stats.serif_count
    );
    println!(
        "║  Sans-Serif:           {:>6}                                        ║",
        stats.sans_serif_count
    );
    println!(
        "║  Display:              {:>6}                                        ║",
        stats.display_count
    );
    println!(
        "║  Handwriting:          {:>6}                                        ║",
        stats.handwriting_count
    );
    println!(
        "║  Monospace:            {:>6}                                        ║",
        stats.monospace_count
    );
    println!(
        "║  Uncategorized:        {:>6}                                        ║",
        stats.uncategorized_count
    );
    println!("╚═══════════════════════════════════════════════════════════════════════╝");
    println!("\n⏱️  Stats calculation time: {:?}", stats_time);
    println!("📦 Providers: {}\n", stats.providers.join(", "));

    // ═══════════════════════════════════════════════════════════════════════════
    // TEST 3: Search Speed Test
    // ═══════════════════════════════════════════════════════════════════════════
    println!("🔍 TEST 3: Search Speed Test (Concurrent)");
    println!("─────────────────────────────────────────────────────────────────────────\n");

    let test_queries = [
        "roboto", "mono", "sans", "serif", "display", "code", "open", "inter",
    ];

    println!("{:<15} {:<10} {:<15}", "Query", "Results", "Time");
    println!("{}", "─".repeat(45));

    for query in &test_queries {
        let (results, elapsed) = search.search_timed(query).await?;
        println!("{:<15} {:<10} {:>8.2?}", query, results.total, elapsed);
    }

    println!("{}", "─".repeat(45));

    // ═══════════════════════════════════════════════════════════════════════════
    // TEST 4: Detailed Search with JSON Output
    // ═══════════════════════════════════════════════════════════════════════════
    println!("\n📋 TEST 4: Detailed Search Results (JSON)");
    println!("─────────────────────────────────────────────────────────────────────────\n");

    let (results, elapsed) = search.search_timed("roboto").await?;
    println!("Search for 'roboto' completed in {:?}", elapsed);
    println!("Found {} fonts matching 'roboto'\n", results.total);

    println!("JSON Response (first 3 results):");
    println!("─────────────────────────────────────────────────────────────────────────");

    let sample: Vec<_> = results.fonts.iter().take(3).collect();
    println!("{}", serde_json::to_string_pretty(&sample)?);

    // ═══════════════════════════════════════════════════════════════════════════
    // TEST 5: CDN URL Generation for Font Preview
    // ═══════════════════════════════════════════════════════════════════════════
    println!("\n\n🌐 TEST 5: CDN URL Generation for Font Preview");
    println!("─────────────────────────────────────────────────────────────────────────\n");

    println!("Popular fonts with CDN preview URLs:\n");

    let popular_fonts = get_popular_font_cdn_urls();

    for (font_name, cdn_urls) in popular_fonts.iter().take(5) {
        println!("📝 {}", font_name);
        println!("   CDN Provider: {:?}", cdn_urls.cdn_provider);
        if let Some(css_url) = &cdn_urls.css_url {
            println!("   CSS URL: {}", css_url);
        }
        if let Some(woff2_url) = &cdn_urls.woff2_url {
            println!("   WOFF2 URL: {}", woff2_url);
        }
        println!();
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TEST 6: Generate Preview HTML for a Font
    // ═══════════════════════════════════════════════════════════════════════════
    println!("🖼️  TEST 6: Preview HTML Generation");
    println!("─────────────────────────────────────────────────────────────────────────\n");

    let font_urls = CdnUrlGenerator::for_google_font("roboto", "Roboto");

    println!("Generated preview HTML for 'Roboto' font:");
    println!("Copy this HTML to a file and open in browser to see the font:\n");

    if let Some(preview_html) = &font_urls.preview_html {
        // Just show a snippet
        let lines: Vec<&str> = preview_html.lines().take(15).collect();
        for line in lines {
            println!("{}", line);
        }
        println!("... (truncated, full HTML has {} lines)", preview_html.lines().count());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TEST 7: Search by Category
    // ═══════════════════════════════════════════════════════════════════════════
    println!("\n\n📂 TEST 7: Search by Font Type");
    println!("─────────────────────────────────────────────────────────────────────────\n");

    for query in [
        "monospace",
        "script",
        "handwriting",
        "gothic",
        "vintage",
        "modern",
    ] {
        let start = Instant::now();
        let results = search.search(query).await?;
        let elapsed = start.elapsed();
        println!("'{}': {} fonts found in {:?}", query, results.total, elapsed);

        // Show top 3
        for font in results.fonts.iter().take(3) {
            println!("  • {} ({})", font.name, font.provider.name());
        }
        println!();
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TEST 8: International Font Search
    // ═══════════════════════════════════════════════════════════════════════════
    println!("🌍 TEST 8: International Font Search");
    println!("─────────────────────────────────────────────────────────────────────────\n");

    for query in [
        "noto", "arabic", "chinese", "japanese", "korean", "hebrew", "cyrillic",
    ] {
        let (results, elapsed) = search.search_timed(query).await?;
        println!("'{}': {} fonts in {:?}", query, results.total, elapsed);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SUMMARY
    // ═══════════════════════════════════════════════════════════════════════════
    println!("\n╔═══════════════════════════════════════════════════════════════════════╗");
    println!("║                      SPEED TEST SUMMARY                               ║");
    println!("╠═══════════════════════════════════════════════════════════════════════╣");
    println!("║  ✅ Concurrent provider fetching: ENABLED                             ║");
    println!("║  ✅ Connection pooling: ENABLED (10 connections/host)                 ║");
    println!("║  ✅ HTTP compression (gzip/brotli): ENABLED                           ║");
    println!("║  ✅ Parallel category counting (Rayon): ENABLED                       ║");
    println!("║  ✅ CDN URL generation: ENABLED                                       ║");
    println!("╚═══════════════════════════════════════════════════════════════════════╝\n");

    println!("🎉 All speed tests completed successfully!");

    Ok(())
}
