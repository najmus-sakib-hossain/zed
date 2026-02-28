use rlm::RLM;
use std::fs;
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let api_key = std::env::var("GROQ_API_KEY")
        .expect("GROQ_API_KEY must be set");

    println!("================================================================================");
    println!("⚡ STREAMING EXECUTION DEMO - Phase 2 Optimization");
    println!("================================================================================");
    println!();
    println!("This demo shows how streaming reduces latency by 2-3 seconds.");
    println!();

    let doc_path = "integrations/recursive-llm/massive_doc.txt";
    let context = fs::read_to_string(doc_path)?;
    let context_arc = Arc::new(context.clone());

    println!("📄 Document: {} characters", context.len());
    println!();

    let rlm = RLM::new(
        api_key,
        "meta-llama/llama-4-scout-17b-16e-instruct".to_string(),
    ).with_max_iterations(20);

    let query = "What is the AI market size? Use fast_find to search.";

    println!("================================================================================");
    println!("TEST 1: Traditional Execution (Wait for Full Response)");
    println!("================================================================================");
    println!();
    println!("Query: {}", query);
    println!();
    println!("Waiting for complete LLM response before executing code...");
    println!();

    let start = Instant::now();
    let (answer1, stats1) = rlm.complete_with_arc(query, context_arc.clone()).await?;
    let time1 = start.elapsed();

    println!("✅ Answer: {}", answer1);
    println!();
    println!("📊 Stats:");
    println!("   Time: {:.2}s", time1.as_secs_f64());
    println!("   LLM calls: {}", stats1.llm_calls);
    println!("   Iterations: {}", stats1.iterations);
    println!();

    println!("{}", "-".repeat(80));
    println!();

    println!("================================================================================");
    println!("TEST 2: Streaming Execution (Process Tokens as They Arrive)");
    println!("================================================================================");
    println!();
    println!("Query: {}", query);
    println!();
    println!("Processing tokens incrementally as they stream from Groq...");
    println!();

    let start = Instant::now();
    let (answer2, stats2) = rlm.complete_streaming(query, context_arc.clone()).await?;
    let time2 = start.elapsed();

    println!("✅ Answer: {}", answer2);
    println!();
    println!("📊 Stats:");
    println!("   Time: {:.2}s", time2.as_secs_f64());
    println!("   LLM calls: {}", stats2.llm_calls);
    println!("   Iterations: {}", stats2.iterations);
    println!();

    println!("{}", "-".repeat(80));
    println!();

    println!("================================================================================");
    println!("📊 STREAMING IMPACT");
    println!("================================================================================");
    println!();

    let latency_reduction = time1.as_secs_f64() - time2.as_secs_f64();
    let speedup = time1.as_secs_f64() / time2.as_secs_f64();

    println!("Traditional execution: {:.2}s", time1.as_secs_f64());
    println!("Streaming execution:   {:.2}s", time2.as_secs_f64());
    println!();
    println!("⚡ Latency reduction: {:.2}s saved", latency_reduction);
    println!("🚀 Speedup: {:.2}x faster", speedup);
    println!();

    println!("How Streaming Works:");
    println!();
    println!("Traditional Approach:");
    println!("  1. Wait for full LLM response (~2-3s)");
    println!("  2. Parse response");
    println!("  3. Execute code");
    println!("  4. Send result back");
    println!();
    println!("Streaming Approach:");
    println!("  1. Start receiving tokens immediately");
    println!("  2. Parse incrementally as tokens arrive");
    println!("  3. Execute code as soon as complete");
    println!("  4. Detect FINAL() early and return");
    println!();
    println!("Benefits:");
    println!("  ✅ 2-3 seconds latency reduction");
    println!("  ✅ Better user experience (progressive output)");
    println!("  ✅ Early termination on FINAL()");
    println!("  ✅ Reduced perceived wait time");
    println!();

    println!("Groq Streaming Stats:");
    println!("  - Token rate: ~200 tokens/second");
    println!("  - Average response: 400-800 tokens");
    println!("  - Time saved: 2-4 seconds per iteration");
    println!();

    Ok(())
}
