// Example: Using OpenCode's free models in DX onboarding
//
// Run with: cargo run --example opencode_demo

use dx_onboard::llm::{
    ChatMessage, ChatRequest, LlmProvider, MessageContent, OPENCODE_FREE_MODELS, OpenCodeProvider,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("┌─ OpenCode Free Models Demo 🚀");
    println!("│");
    println!("│ Powered by OpenCode Zen: https://opencode.ai");
    println!("│");

    // Create OpenCode provider
    let provider = OpenCodeProvider::new()?;
    println!("✓ OpenCode provider initialized");
    println!("│");

    // List available free models
    println!("│ ◇ Available Free Models");
    println!("│");

    match provider.get_models().await {
        Ok(models) => {
            for model in &models {
                let name = model.display_name.as_ref().unwrap_or(&model.id);
                let context = model
                    .context_window
                    .map(|c| format!("{}K context", c / 1000))
                    .unwrap_or_else(|| "Unknown context".to_string());
                println!("  • {} ({})", name, context);
            }
            println!("│");
        }
        Err(e) => {
            println!("  ⚠ Failed to fetch models: {}", e);
            println!("│");
        }
    }

    // Test chat with the default model
    println!("│ ◇ Testing Chat Completion");
    println!("│");
    println!("● Sending request to {}...", OPENCODE_FREE_MODELS[0]);
    println!("│");

    let request = ChatRequest {
        model: OPENCODE_FREE_MODELS[0].to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: MessageContent::Text(
                "Say 'Hello from DX!' in one creative sentence.".to_string(),
            ),
            name: None,
        }],
        temperature: Some(0.7),
        max_tokens: Some(100),
        top_p: None,
        stop: None,
        tools: None,
        tool_choice: None,
        stream: false,
        extra: None,
    };

    match provider.chat(request).await {
        Ok(response) => {
            println!("✓ Response received:");
            println!("│");
            println!("│ {}", response.content);
            println!("│");

            if let Some(usage) = response.usage {
                if let Some(total) = usage.total_tokens {
                    println!("● Tokens used: {}", total);
                }
            }
        }
        Err(e) => {
            println!("✗ Error: {}", e);
        }
    }

    println!("│");
    println!("└─ Demo complete! 🎉");
    println!();
    println!("Try it in your code:");
    println!("  let mut registry = ProviderRegistry::new();");
    println!("  registry.register_openai_compatible_presets();");
    println!("  let provider = registry.get(\"opencode\").unwrap();");

    Ok(())
}
