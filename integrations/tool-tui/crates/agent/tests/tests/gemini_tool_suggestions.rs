use serde_json::json;

#[tokio::test]
async fn ask_gemini_for_advanced_tools() {
    let api_key = std::env::var("GOOGLE_API_KEY").expect("GOOGLE_API_KEY not set");
    let client = reqwest::Client::new();

    let prompt = r#"You are an expert AI agent architect. List 50+ cutting-edge, game-changing tools that an AI coding agent should have in 2026 to be the absolute best. Include:

1. Basic tools (file ops, code execution, web, browser, database, git)
2. Advanced multimodal tools (vision, audio, video processing)
3. Memory systems (vector DB, knowledge graphs, GraphRAG)
4. Workflow automation capabilities
5. Computer use/desktop automation
6. LSP support for all major languages
7. Advanced coding assistant features (multi-file refactoring, test generation, codebase analysis)
8. Security and sandboxing tools
9. Collaboration and communication tools
10. Out-of-the-box innovative tools that would make this agent revolutionary

For each tool, provide:
- Tool name
- Category
- Description (1-2 sentences)
- Why it's game-changing
- Rust crate recommendations (if available)

Format as JSON array."#;

    let response = client
        .post(format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash-exp:generateContent?key={}",
            api_key
        ))
        .json(&json!({
            "contents": [{
                "parts": [{
                    "text": prompt
                }]
            }],
            "generationConfig": {
                "temperature": 0.7,
                "maxOutputTokens": 8192
            }
        }))
        .send()
        .await
        .expect("Failed to send request");

    let result: serde_json::Value = response.json().await.expect("Failed to parse response");

    if let Some(text) = result["candidates"][0]["content"]["parts"][0]["text"].as_str() {
        println!("\n=== GEMINI'S ADVANCED TOOL SUGGESTIONS ===\n");
        println!("{}", text);
        println!("\n=== END SUGGESTIONS ===\n");
    }
}
