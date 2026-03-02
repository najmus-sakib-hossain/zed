//! Test engines that connect but return 0 results
//!
//! Run with: cargo test -p metasearch-engine --test test_empty_engines -- --nocapture

use metasearch_core::query::SearchQuery;
use metasearch_engine::registry::EngineRegistry;
use reqwest::Client;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn test_empty_result_engines() {
    use std::sync::Arc;
    use tokio::sync::Mutex;
    
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (compatible; metasearch-test/0.1)")
        .build()
        .expect("Failed to create HTTP client");

    let registry = Arc::new(EngineRegistry::with_defaults(client));
    
    // Engines that returned 0 results in last test (excluding query-specific ones)
    let empty_engines = vec![
        "duckduckgo", "google", "google_news", "ask", "qwant", "presearch", "yandex",
        "invidious", "piped", "bing_videos",
        "github_code", "searchcode_code", "gitea", "sourcehut",
        "pypi", "fdroid",
        "imgur", "deviantart", "flickr_noapi", "pixiv",
        "ebay", "steam", "huggingface", "naver", "niconico", "nyaa",
        "kickass", "bt4g", "btdigg", "1337x",
    ];

    println!("\n{}", "=".repeat(80));
    println!("  TESTING {} ENGINES IN PARALLEL", empty_engines.len());
    println!("{}", "=".repeat(80));
    println!();

    let now_working = Arc::new(Mutex::new(Vec::new()));
    let still_empty = Arc::new(Mutex::new(Vec::new()));
    let errors = Arc::new(Mutex::new(Vec::new()));
    let completed = Arc::new(Mutex::new(0usize));

    let mut tasks = Vec::new();

    for name in empty_engines.iter() {
        let registry = Arc::clone(&registry);
        let now_working = Arc::clone(&now_working);
        let still_empty = Arc::clone(&still_empty);
        let errors = Arc::clone(&errors);
        let completed = Arc::clone(&completed);
        let name = name.to_string();
        let total = empty_engines.len();

        let task = tokio::spawn(async move {
            let engine = match registry.get(&name) {
                Some(e) => e.clone(),
                None => {
                    let mut errs = errors.lock().await;
                    errs.push((name.clone(), "NOT FOUND".to_string()));
                    return;
                }
            };

            // Try first query
            let query = SearchQuery::new("test");
            let result = tokio::time::timeout(Duration::from_secs(10), engine.search(&query)).await;
            
            match result {
                Ok(Ok(results)) if !results.is_empty() => {
                    let mut working = now_working.lock().await;
                    working.push(name.clone());
                }
                Ok(Ok(_)) => {
                    // Try with another query
                    let query2 = SearchQuery::new("python");
                    match tokio::time::timeout(Duration::from_secs(10), engine.search(&query2)).await {
                        Ok(Ok(results)) if !results.is_empty() => {
                            let mut working = now_working.lock().await;
                            working.push(name.clone());
                        }
                        Ok(Ok(_)) => {
                            let mut empty = still_empty.lock().await;
                            empty.push(name.clone());
                        }
                        Ok(Err(e)) => {
                            let mut errs = errors.lock().await;
                            errs.push((name.clone(), format!("{e}")));
                        }
                        Err(_) => {
                            let mut errs = errors.lock().await;
                            errs.push((name.clone(), "timeout".to_string()));
                        }
                    }
                }
                Ok(Err(e)) => {
                    let mut errs = errors.lock().await;
                    errs.push((name.clone(), format!("{e}")));
                }
                Err(_) => {
                    let mut errs = errors.lock().await;
                    errs.push((name.clone(), "timeout".to_string()));
                }
            }
            
            let mut count = completed.lock().await;
            *count += 1;
            if *count % 5 == 0 || *count == total {
                println!("  Progress: {}/{} engines tested", *count, total);
            }
        });

        tasks.push(task);
    }

    // Wait for all tasks to complete
    for task in tasks {
        let _ = task.await;
    }

    let now_working = now_working.lock().await;
    let still_empty = still_empty.lock().await;
    let errors = errors.lock().await;

    println!();
    println!("{}", "=".repeat(80));
    println!("  RESULTS:");
    println!("    ✅ Now Working:   {}", now_working.len());
    println!("    ⚠️  Still Empty:   {}", still_empty.len());
    println!("    ❌ Errors:        {}", errors.len());
    println!("{}", "=".repeat(80));

    if !now_working.is_empty() {
        println!();
        println!("  Engines that now work:");
        for name in now_working.iter() {
            println!("    • {}", name);
        }
    }

    if !errors.is_empty() {
        println!();
        println!("  Engines with errors:");
        for (name, reason) in errors.iter() {
            let short = if reason.len() > 60 {
                &reason[..60]
            } else {
                reason.as_str()
            };
            println!("    • {}: {}", name, short);
        }
    }

    if !still_empty.is_empty() {
        println!();
        println!("  Engines still returning 0 results (may be query-specific):");
        for name in still_empty.iter() {
            println!("    • {}", name);
        }
    }

    println!();
}
