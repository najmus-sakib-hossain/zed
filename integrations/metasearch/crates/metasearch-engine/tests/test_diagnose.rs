//! Diagnostic test - prints detailed errors for every failing engine
//!
//! Run with: cargo test -p metasearch-engine --test test_diagnose -- --nocapture

use metasearch_core::query::SearchQuery;
use metasearch_engine::registry::EngineRegistry;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[tokio::test(flavor = "multi_thread", worker_threads = 16)]
async fn diagnose_all_engines() {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
        .build()
        .expect("Failed to create HTTP client");

    let registry = Arc::new(EngineRegistry::with_defaults(client));
    let all_engines = registry.engine_names();

    let empty_list: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let error_list: Arc<Mutex<Vec<(String, String)>>> = Arc::new(Mutex::new(Vec::new()));
    let working_count = Arc::new(Mutex::new(0usize));

    let mut tasks = Vec::new();

    for name in all_engines.iter() {
        let registry = Arc::clone(&registry);
        let empty_list = Arc::clone(&empty_list);
        let error_list = Arc::clone(&error_list);
        let working_count = Arc::clone(&working_count);
        let name = name.to_string();

        let task = tokio::spawn(async move {
            let engine = match registry.get(&name) {
                Some(e) => e,
                None => return,
            };

            let query = SearchQuery::new("test");
            let result = tokio::time::timeout(Duration::from_secs(8), engine.search(&query)).await;
            
            match result {
                Ok(Ok(results)) if !results.is_empty() => {
                    let mut w = working_count.lock().await;
                    *w += 1;
                }
                Ok(Ok(_)) => {
                    let mut e = empty_list.lock().await;
                    e.push(name.clone());
                }
                Ok(Err(err)) => {
                    let mut e = error_list.lock().await;
                    e.push((name.clone(), format!("{err}")));
                }
                Err(_) => {
                    let mut e = error_list.lock().await;
                    e.push((name.clone(), "TIMEOUT (8s)".to_string()));
                }
            }
        });

        tasks.push(task);
    }

    for task in tasks {
        let _ = task.await;
    }

    let empty_list = empty_list.lock().await;
    let error_list = error_list.lock().await;
    let working = *working_count.lock().await;

    println!("\n=== WORKING: {} ===", working);
    
    println!("\n=== EMPTY ({}) ===", empty_list.len());
    let mut sorted_empty = empty_list.clone();
    sorted_empty.sort();
    for name in &sorted_empty {
        println!("  EMPTY: {}", name);
    }
    
    println!("\n=== ERRORS ({}) ===", error_list.len());
    let mut sorted_errors = error_list.clone();
    sorted_errors.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, err) in &sorted_errors {
        println!("  ERROR: {} => {}", name, err);
    }
}
