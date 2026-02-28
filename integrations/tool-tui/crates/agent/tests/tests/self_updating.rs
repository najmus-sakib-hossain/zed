//! Self-Updating Tests
//!
//! Tests agent's ability to learn and improve using Gemini.

use anyhow::Result;
use google_gemini_rs::{Client, GenerateContentRequest};
use serde_json::json;

fn load_api_key() -> Result<String> {
    dotenvy::from_filename("../../../.env").ok();
    std::env::var("GOOGLE_API_KEY").map_err(|_| anyhow::anyhow!("GOOGLE_API_KEY not found"))
}

#[tokio::test]
async fn test_capability_detection() -> Result<()> {
    let client = Client::new(&load_api_key()?);

    let request = GenerateContentRequest::new("gemini-2.5-flash").with_prompt(
        r#"
Analyze this user request and identify what capabilities are needed:
"Send a message to my Telegram group and then create a summary document"

Return JSON with: {"capabilities": ["capability1", "capability2"]}
"#,
    );

    let response = client.generate_content(request).await?;
    let text = response.text();

    println!("✓ Capability detection test");
    println!("Detected capabilities: {}", text);

    // Should detect telegram and file writing capabilities
    assert!(text.to_lowercase().contains("telegram") || text.to_lowercase().contains("message"));

    Ok(())
}

#[tokio::test]
async fn test_skill_generation() -> Result<()> {
    let client = Client::new(&load_api_key()?);

    let request = GenerateContentRequest::new("gemini-2.5-flash").with_prompt(
        r#"
Generate a skill configuration for a "weather" skill that:
1. Fetches weather data from an API
2. Formats it nicely
3. Returns temperature and conditions

Return as JSON with fields: name, description, parameters, implementation_hints
"#,
    );

    let response = client.generate_content(request).await?;
    let text = response.text();

    println!("✓ Skill generation test");
    println!("Generated skill: {}", text);

    assert!(text.contains("weather") || text.contains("temperature"));

    Ok(())
}

#[tokio::test]
async fn test_error_analysis() -> Result<()> {
    let client = Client::new(&load_api_key()?);

    let error_log = r#"
Error: Failed to connect to Telegram
Cause: Network timeout after 30s
Stack trace: ...
"#;

    let request = GenerateContentRequest::new("gemini-2.5-flash").with_prompt(&format!(
        r#"
Analyze this error and suggest fixes:
{}

Return JSON with: {{"problem": "...", "solutions": ["solution1", "solution2"]}}
"#,
        error_log
    ));

    let response = client.generate_content(request).await?;
    let text = response.text();

    println!("✓ Error analysis test");
    println!("Analysis: {}", text);

    assert!(text.to_lowercase().contains("timeout") || text.to_lowercase().contains("network"));

    Ok(())
}

#[tokio::test]
async fn test_configuration_generation() -> Result<()> {
    let client = Client::new(&load_api_key()?);

    let request = GenerateContentRequest::new("gemini-2.5-flash").with_prompt(
        r#"
Generate a configuration file for a Telegram bot with:
- Bot token placeholder
- Allowed users list
- Rate limiting settings
- Logging level

Return as TOML format.
"#,
    );

    let response = client.generate_content(request).await?;
    let text = response.text();

    println!("✓ Configuration generation test");
    println!("Generated config:\n{}", text);

    assert!(text.contains("token") || text.contains("bot"));

    Ok(())
}

#[tokio::test]
async fn test_code_improvement_suggestions() -> Result<()> {
    let client = Client::new(&load_api_key()?);

    let code = r#"
fn process_data(data: Vec<String>) -> Vec<String> {
    let mut result = Vec::new();
    for item in data {
        result.push(item.to_uppercase());
    }
    result
}
"#;

    let request = GenerateContentRequest::new("gemini-2.5-flash").with_prompt(&format!(
        r#"
Analyze this Rust code and suggest improvements:
{}

Focus on: performance, idiomatic Rust, error handling.
Return as JSON with: {{"improvements": ["improvement1", "improvement2"]}}
"#,
        code
    ));

    let response = client.generate_content(request).await?;
    let text = response.text();

    println!("✓ Code improvement test");
    println!("Suggestions: {}", text);

    // Should suggest using iterators or map
    assert!(text.to_lowercase().contains("iter") || text.to_lowercase().contains("map"));

    Ok(())
}

#[tokio::test]
async fn test_learning_from_feedback() -> Result<()> {
    let client = Client::new(&load_api_key()?);

    let feedback = r#"
User feedback: "The response was too verbose. Please be more concise."
Previous response: "Well, let me explain in detail. First, we need to understand..."
"#;

    let request = GenerateContentRequest::new("gemini-2.5-flash").with_prompt(&format!(
        r#"
Learn from this feedback and generate an improved response style guide:
{}

Return JSON with: {{"style_rules": ["rule1", "rule2"], "example": "..."}}
"#,
        feedback
    ));

    let response = client.generate_content(request).await?;
    let text = response.text();

    println!("✓ Learning from feedback test");
    println!("Learned rules: {}", text);

    assert!(text.to_lowercase().contains("concise") || text.to_lowercase().contains("brief"));

    Ok(())
}

#[tokio::test]
async fn test_adaptive_prompting() -> Result<()> {
    let client = Client::new(&load_api_key()?);

    // First attempt - vague
    let request1 =
        GenerateContentRequest::new("gemini-2.5-flash").with_prompt("Tell me about Rust");

    let response1 = client.generate_content(request1).await?;

    // Second attempt - more specific based on first response
    let request2 = GenerateContentRequest::new("gemini-2.5-flash").with_prompt(&format!(
        r#"
Previous response was: "{}"
User wants more specific info about Rust's memory safety.
Provide a focused response.
"#,
        response1.text()
    ));

    let response2 = client.generate_content(request2).await?;

    println!("✓ Adaptive prompting test");
    println!("First: {}", response1.text().chars().take(100).collect::<String>());
    println!("Adapted: {}", response2.text().chars().take(100).collect::<String>());

    assert!(
        response2.text().to_lowercase().contains("memory")
            || response2.text().to_lowercase().contains("safety")
    );

    Ok(())
}
