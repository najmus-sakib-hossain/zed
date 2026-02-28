use dotenv::dotenv;
use dx_agent_tools::Tool; // Import the trait
use dx_agent_tools::definition::ToolCall;
use dx_agent_tools::llm::LlmTool;
use serde_json::json;
use std::env;

#[tokio::test]
async fn test_groq_integration() {
    // Load .env from workspace root
    // Try to find .env by walking up directories
    let _ = dotenv();
    let root_path = std::path::Path::new("f:/Dx/.env");
    if root_path.exists() {
        dotenv::from_path(root_path).ok();
    }

    if let Ok(key) = env::var("GROQ_API_KEY") {
        println!("🔑 GROQ_API_KEY found: {}...", key.chars().take(4).collect::<String>());
    } else {
        eprintln!("⚠️ GROQ_API_KEY not found in env. Test may fail if not set globally.");
    }

    let tool = LlmTool::default();

    // Use the requested model
    let model = "meta-llama/llama-prompt-guard-2-86m";
    let provider = "groq";

    println!("Testing Groq with model: {}", model);

    let call = ToolCall {
        id: "groq-test-1".to_string(),
        name: "llm".into(),
        arguments: json!({
            "action": "chat",
            "provider": provider,
            "model": model,
            "messages": [
                { "role": "user", "content": "Just say 'working'" }
            ],
            "max_tokens": 10
        }),
    };

    match tool.execute(call).await {
        Ok(res) => {
            if res.success {
                println!("✅ Groq Test Passed!");
                println!("Response: {}", res.output);
            } else {
                let err = res.error.unwrap_or_else(|| "Unknown error".to_string());
                panic!("❌ Groq Test Failed (Tool Error): {}", err);
            }
        }
        Err(e) => {
            panic!("❌ Groq Test Failed (Execution Error): {:?}", e);
        }
    }
}
