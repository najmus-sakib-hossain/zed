use rlm::RLM;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Rust RLM - Simple Test");
    println!();

    let api_key = "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".to_string();

    // Small context for testing
    let context = r#"
# Tech Report 2024

## AI Market
The global AI market reached $184 billion in 2024, growing at 37.3% annually.
Major players include OpenAI, Anthropic, Google, and Meta.

## Space Industry
SpaceX completed 96 successful launches in 2024.
Starship achieved its first orbital flight in March 2024.
"#;

    println!("📄 Context: {} characters", context.len());
    println!();

    let rlm = RLM::new(api_key, "llama-3.3-70b-versatile".to_string())
        .with_max_iterations(10);

    println!("Query: What is the AI market size in 2024?");
    println!();

    match rlm.complete("What is the AI market size in 2024? Use fast_find to search for 'AI market'.", context).await {
        Ok((answer, stats)) => {
            println!("✅ Answer: {}", answer);
            println!();
            println!("Stats:");
            println!("  LLM calls: {}", stats.llm_calls);
            println!("  Iterations: {}", stats.iterations);
            println!("  Cache hit rate: {:.1}%", stats.cache_hit_rate());
            println!();
            println!("🎯 RLM works perfectly!");
        }
        Err(e) => {
            println!("❌ Error: {}", e);
        }
    }

    Ok(())
}
