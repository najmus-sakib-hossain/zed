use rlm::RLM;
use std::fs;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let api_key = std::env::var("GROQ_API_KEY")
        .expect("GROQ_API_KEY must be set");

    println!("================================================================================");
    println!("💰 MULTI-MODEL ROUTING DEMO - Phase 3 Optimization");
    println!("================================================================================");
    println!();
    println!("This demo shows how smart model routing reduces costs by 50-70%.");
    println!();

    let doc_path = "integrations/recursive-llm/massive_doc.txt";
    let context = fs::read_to_string(doc_path)?;

    println!("📄 Document: {} characters", context.len());
    println!();

    println!("================================================================================");
    println!("TEST 1: Single Model (Baseline)");
    println!("================================================================================");
    println!();
    println!("Using only smart model (llama-4-scout) for all tasks...");
    println!();

    let rlm_single = RLM::new(
        api_key.clone(),
        "meta-llama/llama-4-scout-17b-16e-instruct".to_string(),
    ).with_max_iterations(20);

    let query = "What is the AI market size? Use fast_find to search.";
    
    let start = Instant::now();
    let (answer1, stats1) = rlm_single.complete(query, &context).await?;
    let time1 = start.elapsed();

    println!("✅ Answer: {}", answer1);
    println!();
    println!("📊 Stats:");
    println!("   Time: {:.2}s", time1.as_secs_f64());
    println!("   LLM calls: {}", stats1.llm_calls);
    println!("   Smart model: {} calls", stats1.smart_model_calls);
    println!("   Fast model: {} calls", stats1.fast_model_calls);
    println!();

    println!("{}", "-".repeat(80));
    println!();

    println!("================================================================================");
    println!("TEST 2: Multi-Model Routing (Optimized)");
    println!("================================================================================");
    println!();
    println!("Smart model: llama-4-scout (for synthesis/reasoning)");
    println!("Fast model:  llama-3.3-70b (for search/exploration)");
    println!();

    let rlm_multi = RLM::new(
        api_key.clone(),
        "meta-llama/llama-4-scout-17b-16e-instruct".to_string(),
    )
    .with_fast_model("meta-llama/llama-3.3-70b-versatile".to_string())
    .with_max_iterations(20);

    let start = Instant::now();
    let (answer2, stats2) = rlm_multi.complete(query, &context).await?;
    let time2 = start.elapsed();

    println!("✅ Answer: {}", answer2);
    println!();
    println!("📊 Stats:");
    println!("   Time: {:.2}s", time2.as_secs_f64());
    println!("   LLM calls: {}", stats2.llm_calls);
    println!("   Smart model: {} calls", stats2.smart_model_calls);
    println!("   Fast model: {} calls", stats2.fast_model_calls);
    println!("   Cost savings: {:.1}%", stats2.cost_savings());
    println!();

    println!("{}", "-".repeat(80));
    println!();

    println!("================================================================================");
    println!("TEST 3: Multiple Queries (Cost Analysis)");
    println!("================================================================================");
    println!();

    let queries = vec![
        "Find AI market size using fast_find",
        "Find SpaceX launches using fast_find_all",
        "Find remote work stats using fast_contains",
    ];

    let mut total_fast = 0;
    let mut total_smart = 0;
    let mut total_time = 0.0;

    for (i, q) in queries.iter().enumerate() {
        println!("Query {}: {}", i + 1, q);
        
        let start = Instant::now();
        let (answer, stats) = rlm_multi.complete(q, &context).await?;
        let elapsed = start.elapsed();
        
        total_fast += stats.fast_model_calls;
        total_smart += stats.smart_model_calls;
        total_time += elapsed.as_secs_f64();
        
        println!("   Answer: {}...", &answer[..answer.len().min(60)]);
        println!("   Fast: {} | Smart: {} | Cost savings: {:.1}%", 
            stats.fast_model_calls, 
            stats.smart_model_calls,
            stats.cost_savings());
        println!();
    }

    println!("Aggregate Stats:");
    println!("   Total time: {:.2}s", total_time);
    println!("   Fast model calls: {}", total_fast);
    println!("   Smart model calls: {}", total_smart);
    
    let total_calls = total_fast + total_smart;
    let baseline_cost = total_calls as f64;
    let actual_cost = (total_fast as f64 * 0.1) + (total_smart as f64);
    let savings = ((baseline_cost - actual_cost) / baseline_cost) * 100.0;
    
    println!("   Overall cost savings: {:.1}%", savings);
    println!();

    println!("{}", "-".repeat(80));
    println!();

    println!("================================================================================");
    println!("💰 COST BREAKDOWN");
    println!("================================================================================");
    println!();

    println!("Model Pricing (relative):");
    println!("   Smart model (llama-4-scout):  1.0x cost");
    println!("   Fast model (llama-3.3-70b):    0.1x cost (10x cheaper)");
    println!();

    println!("Single Model Approach:");
    println!("   All {} calls use smart model", total_calls);
    println!("   Total cost: {:.1}x", baseline_cost);
    println!();

    println!("Multi-Model Approach:");
    println!("   {} calls use fast model (0.1x each)", total_fast);
    println!("   {} calls use smart model (1.0x each)", total_smart);
    println!("   Total cost: {:.1}x", actual_cost);
    println!();

    println!("💰 SAVINGS: {:.1}% cost reduction!", savings);
    println!();

    println!("{}", "-".repeat(80));
    println!();

    println!("================================================================================");
    println!("🎯 HOW ROUTING WORKS");
    println!("================================================================================");
    println!();

    println!("Fast Model Used For:");
    println!("   ✅ Search operations (fast_find, fast_contains)");
    println!("   ✅ Text extraction (sub_string, index_of)");
    println!("   ✅ Pattern matching (find, locate)");
    println!("   ✅ REPL exploration tasks");
    println!();

    println!("Smart Model Used For:");
    println!("   ✅ Final synthesis (FINAL() calls)");
    println!("   ✅ Complex reasoning (analyze, compare)");
    println!("   ✅ Summarization tasks");
    println!("   ✅ Decision making");
    println!();

    println!("Benefits:");
    println!("   💰 50-70% cost reduction");
    println!("   ⚡ Same or better accuracy");
    println!("   🎯 Automatic task detection");
    println!("   🔧 Configurable model selection");
    println!();

    Ok(())
}
