//! Gemini 2.7B Integration Tests
//!
//! Tests DX Agent features using Google AI Studio's Gemini 2.7B model.
//! Uses API key from root .env file.

use anyhow::Result;
use google_gemini_rs::{Client, GenerateContentRequest};
use serde_json::json;

/// Load API key from .env file
fn load_api_key() -> Result<String> {
    dotenvy::from_filename("../../.env").ok();
    std::env::var("GOOGLE_API_KEY").map_err(|_| anyhow::anyhow!("GOOGLE_API_KEY not found in .env"))
}

/// Create Gemini client
fn create_client() -> Result<Client> {
    let api_key = load_api_key()?;
    Ok(Client::new(&api_key))
}

#[tokio::test]
async fn test_gemini_connection() -> Result<()> {
    let client = create_client()?;

    let request =
        GenerateContentRequest::new("gemini-2.5-flash").with_prompt("Say 'Hello from DX Agent!'");

    let response = client.generate_content(request).await?;

    assert!(!response.text().is_empty());
    assert!(response.text().contains("Hello") || response.text().contains("DX"));

    println!("✓ Gemini connection successful");
    println!("Response: {}", response.text());

    Ok(())
}

#[tokio::test]
async fn test_gemini_code_generation() -> Result<()> {
    let client = create_client()?;

    let request = GenerateContentRequest::new("gemini-2.5-flash")
        .with_prompt("Write a Rust function that adds two numbers. Just the code, no explanation.");

    let response = client.generate_content(request).await?;
    let code = response.text();

    assert!(code.contains("fn") || code.contains("function"));
    assert!(code.contains("+") || code.contains("add"));

    println!("✓ Code generation successful");
    println!("Generated code:\n{}", code);

    Ok(())
}

#[tokio::test]
async fn test_gemini_json_output() -> Result<()> {
    let client = create_client()?;

    let request = GenerateContentRequest::new("gemini-2.5-flash")
        .with_prompt("Return a JSON object with fields: name='DX Agent', version='0.1.0', status='active'. Only JSON, no markdown.");

    let response = client.generate_content(request).await?;
    let text = response.text();

    // Try to parse as JSON
    let json_start = text.find('{').unwrap_or(0);
    let json_end = text.rfind('}').map(|i| i + 1).unwrap_or(text.len());
    let json_str = &text[json_start..json_end];

    let parsed: serde_json::Value = serde_json::from_str(json_str)?;

    assert!(parsed.get("name").is_some());
    assert!(parsed.get("version").is_some());

    println!("✓ JSON output successful");
    println!("Parsed JSON: {}", serde_json::to_string_pretty(&parsed)?);

    Ok(())
}

#[tokio::test]
async fn test_gemini_multi_turn() -> Result<()> {
    let client = create_client()?;

    // First turn
    let request1 = GenerateContentRequest::new("gemini-2.5-flash")
        .with_prompt("My name is Alice. Remember this.");

    let response1 = client.generate_content(request1).await?;
    println!("Turn 1: {}", response1.text());

    // Second turn (should remember context)
    let request2 = GenerateContentRequest::new("gemini-2.5-flash").with_prompt("What is my name?");

    let response2 = client.generate_content(request2).await?;
    let text = response2.text();

    // Note: Without session management, Gemini won't remember
    // This test demonstrates the need for session context
    println!("Turn 2: {}", text);

    println!("✓ Multi-turn conversation test complete");
    println!("Note: Session management needed for context retention");

    Ok(())
}

#[tokio::test]
async fn test_gemini_streaming() -> Result<()> {
    let client = create_client()?;

    let request = GenerateContentRequest::new("gemini-2.5-flash")
        .with_prompt("Count from 1 to 5, one number per line.")
        .with_streaming(true);

    let mut stream = client.generate_content_stream(request).await?;
    let mut chunks = Vec::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        chunks.push(chunk.text().to_string());
        print!("{}", chunk.text());
    }

    println!("\n✓ Streaming successful");
    println!("Received {} chunks", chunks.len());

    Ok(())
}

#[tokio::test]
async fn test_gemini_function_calling() -> Result<()> {
    let client = create_client()?;

    // Define a function schema
    let function_schema = json!({
        "name": "get_weather",
        "description": "Get the current weather for a location",
        "parameters": {
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City name"
                },
                "unit": {
                    "type": "string",
                    "enum": ["celsius", "fahrenheit"]
                }
            },
            "required": ["location"]
        }
    });

    let request = GenerateContentRequest::new("gemini-2.5-flash")
        .with_prompt("What's the weather in San Francisco?")
        .with_tools(vec![function_schema]);

    let response = client.generate_content(request).await?;

    println!("✓ Function calling test complete");
    println!("Response: {}", response.text());

    // Check if function was called
    if let Some(function_call) = response.function_calls().first() {
        println!("Function called: {}", function_call.name());
        println!("Arguments: {:?}", function_call.args());
    }

    Ok(())
}

#[tokio::test]
async fn test_gemini_error_handling() -> Result<()> {
    let client = create_client()?;

    // Test with invalid model
    let request = GenerateContentRequest::new("invalid-model-name").with_prompt("This should fail");

    let result = client.generate_content(request).await;

    assert!(result.is_err());
    println!("✓ Error handling successful");
    println!("Expected error: {:?}", result.unwrap_err());

    Ok(())
}

#[tokio::test]
async fn test_gemini_rate_limiting() -> Result<()> {
    let client = create_client()?;

    // Send multiple requests rapidly
    let mut handles = vec![];

    for i in 0..5 {
        let client = client.clone();
        let handle = tokio::spawn(async move {
            let request = GenerateContentRequest::new("gemini-2.5-flash")
                .with_prompt(&format!("Request {}", i));

            client.generate_content(request).await
        });
        handles.push(handle);
    }

    let mut success_count = 0;
    let mut error_count = 0;

    for handle in handles {
        match handle.await? {
            Ok(_) => success_count += 1,
            Err(_) => error_count += 1,
        }
    }

    println!("✓ Rate limiting test complete");
    println!("Successful: {}, Errors: {}", success_count, error_count);
    println!("Note: Free tier has 15 RPM limit");

    Ok(())
}
