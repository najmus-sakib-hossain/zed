//! Integration test: probe every registered engine with a real search query.
//!
//! Run locally:
//!   cargo test -p metasearch-engine --test engine_probe -- --ignored --nocapture
//!
//! This test is `#[ignore]` by default because it hits real external APIs.
//! The CI workflow runs it explicitly in a dedicated job.

use metasearch_core::query::SearchQuery;
use metasearch_engine::registry::EngineRegistry;
use reqwest::Client;
use std::time::Duration;

#[tokio::test]
#[ignore]
async fn probe_all_engines() {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (compatible; metasearch-ci/0.1)")
        .build()
        .expect("Failed to create HTTP client");

    let registry = EngineRegistry::with_defaults(client);
    let query = SearchQuery::new("rust programming");

    let mut passed: Vec<String> = Vec::new();
    let mut failed: Vec<(String, String)> = Vec::new();

    let mut engine_names = registry.engine_names();
    engine_names.sort();
    let total = engine_names.len();

    println!();
    println!("{}", "=".repeat(70));
    println!(
        "  ENGINE PROBE \u{2014} testing {} engines with query \"rust programming\"",
        total
    );
    println!("{}", "=".repeat(70));
    println!();

    for (i, name) in engine_names.iter().enumerate() {
        let engine = match registry.get(name) {
            Some(e) => e.clone(),
            None => {
                failed.push((name.clone(), "not found in registry".into()));
                continue;
            }
        };
        let meta = engine.metadata();

        print!("  [{:>3}/{}] {:<30} ", i + 1, total, meta.display_name);

        match tokio::time::timeout(Duration::from_secs(15), engine.search(&query)).await {
            Ok(Ok(results)) if !results.is_empty() => {
                println!("\u{2705}  {} result(s)", results.len());
                passed.push(name.clone());
            }
            Ok(Ok(_)) => {
                println!("\u{26a0}\u{fe0f}   0 results (empty)");
                failed.push((name.clone(), "empty results".into()));
            }
            Ok(Err(e)) => {
                let msg = format!("{e}");
                let short = if msg.len() > 60 { &msg[..60] } else { &msg };
                println!("\u{274c}  {short}");
                failed.push((name.clone(), msg));
            }
            Err(_) => {
                println!("\u{23f1}\u{fe0f}   timeout (15s)");
                failed.push((name.clone(), "timeout (15s)".into()));
            }
        }
    }

    println!();
    println!("{}", "=".repeat(70));
    println!(
        "  RESULTS: {} / {} passed,  {} / {} failed",
        passed.len(),
        total,
        failed.len(),
        total,
    );
    println!("{}", "=".repeat(70));

    if !failed.is_empty() {
        println!();
        println!("  Failed engines:");
        for (name, reason) in &failed {
            let short = if reason.len() > 60 {
                &reason[..60]
            } else {
                reason.as_str()
            };
            println!("    \u{2022} {name}: {short}");
        }
    }

    println!();

    // Soft pass: at least 30% of engines should return results.
    // External services can be flaky in CI environments.
    let pass_rate = passed.len() as f64 / total as f64;
    assert!(
        pass_rate >= 0.3,
        "Only {:.0}% of engines returned results (expected >= 30%)",
        pass_rate * 100.0
    );
}
