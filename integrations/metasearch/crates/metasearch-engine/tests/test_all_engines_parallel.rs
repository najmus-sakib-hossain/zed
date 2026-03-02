//! Test ALL engines in parallel for maximum speed
//!
//! Run with: cargo test -p metasearch-engine --test test_all_engines_parallel -- --nocapture

use metasearch_core::query::SearchQuery;
use metasearch_engine::registry::EngineRegistry;
use reqwest::Client;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[tokio::test(flavor = "multi_thread", worker_threads = 16)]
async fn test_all_engines_parallel() {
    let start_time = Instant::now();
    
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (compatible; metasearch-test/0.1)")
        .build()
        .expect("Failed to create HTTP client");

    let registry = Arc::new(EngineRegistry::with_defaults(client));
    let all_engines = registry.engine_names();

    println!("\n{}", "=".repeat(80));
    println!("  TESTING ALL {} ENGINES IN PARALLEL", all_engines.len());
    println!("  Using 16 worker threads for maximum speed");
    println!("{}", "=".repeat(80));
    println!();

    let working: Arc<Mutex<Vec<(String, usize)>>> = Arc::new(Mutex::new(Vec::new()));
    let empty: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let errors: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));
    let completed = Arc::new(Mutex::new(0usize));

    let mut tasks = Vec::new();

    for name in all_engines.iter() {
        let registry = Arc::clone(&registry);
        let working = Arc::clone(&working);
        let empty = Arc::clone(&empty);
        let errors = Arc::clone(&errors);
        let completed = Arc::clone(&completed);
        let name = name.to_string();
        let total = all_engines.len();

        let task = tokio::spawn(async move {
            let engine = match registry.get(&name) {
                Some(e) => e,
                None => return,
            };

            // Try with a simple query
            let query = SearchQuery::new("test");
            let result = tokio::time::timeout(Duration::from_secs(8), engine.search(&query)).await;
            
            match result {
                Ok(Ok(results)) if !results.is_empty() => {
                    let mut w = working.lock().await;
                    w.push((name.clone(), results.len()));
                }
                Ok(Ok(_)) => {
                    let mut e = empty.lock().await;
                    e.push(name.clone());
                }
                Ok(Err(err)) => {
                    let mut e = errors.lock().await;
                    e.push((name.clone(), format!("{err}")));
                }
                Err(_) => {
                    let mut e = errors.lock().await;
                    e.push((name.clone(), "timeout".to_string()));
                }
            }
            
            let mut count = completed.lock().await;
            *count += 1;
            if *count % 20 == 0 || *count == total {
                println!("  Progress: {}/{} engines tested ({:.1}%)", 
                    *count, total, (*count as f64 / total as f64) * 100.0);
            }
        });

        tasks.push(task);
    }

    // Wait for all tasks to complete
    for task in tasks {
        let _ = task.await;
    }

    let working = working.lock().await;
    let empty = empty.lock().await;
    let errors = errors.lock().await;
    
    let elapsed = start_time.elapsed();

    println!();
    println!("{}", "=".repeat(80));
    println!("  BRUTAL TRUTH - FINAL RESULTS:");
    println!("  ⏱️  Total time: {:.2}s (avg {:.2}s per engine)", 
        elapsed.as_secs_f64(), 
        elapsed.as_secs_f64() / all_engines.len() as f64);
    println!();
    println!("  ✅ WORKING:      {} engines ({:.1}%)", 
        working.len(), 
        (working.len() as f64 / all_engines.len() as f64) * 100.0);
    println!("  ⚠️  EMPTY:        {} engines ({:.1}%)", 
        empty.len(), 
        (empty.len() as f64 / all_engines.len() as f64) * 100.0);
    println!("  ❌ ERRORS:       {} engines ({:.1}%)", 
        errors.len(), 
        (errors.len() as f64 / all_engines.len() as f64) * 100.0);
    println!("{}", "=".repeat(80));

    // Show top 10 working engines by result count
    let mut sorted_working = working.clone();
    sorted_working.sort_by(|a, b| b.1.cmp(&a.1));
    
    println!();
    println!("  🏆 TOP 10 WORKING ENGINES (by result count):");
    for (i, (name, count)) in sorted_working.iter().take(10).enumerate() {
        println!("    {}. {:<30} {} results", i + 1, name, count);
    }

    // Show error breakdown
    if !errors.is_empty() {
        println!();
        println!("  ❌ ERROR BREAKDOWN:");
        
        let mut error_types: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        for (name, err) in errors.iter() {
            let err_type = if err.contains("timeout") {
                "Timeout"
            } else if err.contains("Parse") {
                "Parse Error"
            } else if err.contains("Http") || err.contains("Network") {
                "Network Error"
            } else if err.contains("RateLimited") {
                "Rate Limited"
            } else {
                "Other"
            };
            error_types.entry(err_type.to_string()).or_default().push(name.clone());
        }
        
        for (err_type, engines) in error_types.iter() {
            println!("    • {}: {} engines", err_type, engines.len());
        }
    }

    // Show empty results engines
    if !empty.is_empty() {
        println!();
        println!("  ⚠️  ENGINES RETURNING 0 RESULTS:");
        println!("    (These may have bot protection or need specific queries)");
        for (i, name) in empty.iter().enumerate() {
            print!("    {:<25}", name);
            if (i + 1) % 3 == 0 {
                println!();
            }
        }
        println!();
    }

    if !errors.is_empty() {
        println!();
        println!("  ❌ ENGINES WITH ERRORS:");
        for (name, err) in errors.iter() {
            println!("    {:<25}: {}", name, err);
        }
        println!();
    }

    println!();
    println!("  📊 SUMMARY:");
    println!("    • Total engines: {}", all_engines.len());
    println!("    • Success rate: {:.1}%", (working.len() as f64 / all_engines.len() as f64) * 100.0);
    println!("    • Potential rate (if empty fixed): {:.1}%", 
        ((working.len() + empty.len()) as f64 / all_engines.len() as f64) * 100.0);
    println!("    • Speed: {:.1}x faster than sequential", 
        (all_engines.len() as f64 * 8.0) / elapsed.as_secs_f64());
    println!("{}", "=".repeat(80));
    println!();
}
