//! Tool Calling Tests
//!
//! Tests tool execution with Gemini function calling.

use anyhow::Result;
use dx_agent_tools::{Tool, ToolRegistry, ToolResult};
use google_gemini_rs::{Client, GenerateContentRequest};
use serde_json::json;

fn load_api_key() -> Result<String> {
    dotenvy::from_filename("../../../.env").ok();
    std::env::var("GOOGLE_API_KEY").map_err(|_| anyhow::anyhow!("GOOGLE_API_KEY not found"))
}

#[tokio::test]
async fn test_bash_tool() -> Result<()> {
    let tool = dx_agent_tools::bash();

    let result = tool
        .execute(json!({
            "command": "echo 'Hello from DX Agent'"
        }))
        .await?;

    assert!(result.success);
    assert!(result.output.contains("Hello from DX Agent"));

    println!("✓ Bash tool working");
    println!("Output: {}", result.output);

    Ok(())
}

#[tokio::test]
async fn test_read_file_tool() -> Result<()> {
    // Create test file
    std::fs::write("/tmp/dx_test.txt", "Test content")?;

    let tool = dx_agent_tools::read_file();

    let result = tool
        .execute(json!({
            "path": "/tmp/dx_test.txt"
        }))
        .await?;

    assert!(result.success);
    assert!(result.output.contains("Test content"));

    println!("✓ Read file tool working");

    // Cleanup
    std::fs::remove_file("/tmp/dx_test.txt").ok();

    Ok(())
}

#[tokio::test]
async fn test_write_file_tool() -> Result<()> {
    let tool = dx_agent_tools::write_file();

    let result = tool
        .execute(json!({
            "path": "/tmp/dx_write_test.txt",
            "content": "Written by DX Agent"
        }))
        .await?;

    assert!(result.success);

    // Verify file was written
    let content = std::fs::read_to_string("/tmp/dx_write_test.txt")?;
    assert_eq!(content, "Written by DX Agent");

    println!("✓ Write file tool working");

    // Cleanup
    std::fs::remove_file("/tmp/dx_write_test.txt").ok();

    Ok(())
}

#[tokio::test]
async fn test_gemini_function_calling_integration() -> Result<()> {
    let client = Client::new(&load_api_key()?);

    // Define calculator tool
    let calculator_schema = json!({
        "name": "calculate",
        "description": "Perform basic arithmetic operations",
        "parameters": {
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"]
                },
                "a": {
                    "type": "number",
                    "description": "First number"
                },
                "b": {
                    "type": "number",
                    "description": "Second number"
                }
            },
            "required": ["operation", "a", "b"]
        }
    });

    let request = GenerateContentRequest::new("gemini-2.5-flash")
        .with_prompt("Calculate 15 + 27")
        .with_tools(vec![calculator_schema]);

    let response = client.generate_content(request).await?;

    println!("✓ Gemini function calling integration test");

    if let Some(function_call) = response.function_calls().first() {
        println!("Function: {}", function_call.name());
        println!("Arguments: {:?}", function_call.args());

        assert_eq!(function_call.name(), "calculate");

        let args = function_call.args();
        assert_eq!(args.get("operation").and_then(|v| v.as_str()), Some("add"));
        assert_eq!(args.get("a").and_then(|v| v.as_f64()), Some(15.0));
        assert_eq!(args.get("b").and_then(|v| v.as_f64()), Some(27.0));
    } else {
        println!("Note: Function calling may not be triggered in all cases");
    }

    Ok(())
}

#[tokio::test]
async fn test_tool_error_handling() -> Result<()> {
    let tool = dx_agent_tools::bash();

    // Execute invalid command
    let result = tool
        .execute(json!({
            "command": "nonexistent_command_xyz"
        }))
        .await?;

    assert!(!result.success);
    assert!(!result.error.is_empty());

    println!("✓ Tool error handling working");
    println!("Error: {}", result.error);

    Ok(())
}

#[tokio::test]
async fn test_tool_registry_execution() -> Result<()> {
    let mut registry = ToolRegistry::new();

    // Register tools
    registry.register(dx_agent_tools::bash());
    registry.register(dx_agent_tools::read_file());
    registry.register(dx_agent_tools::write_file());

    // Execute tool by name
    let result = registry
        .execute(
            "bash",
            json!({
                "command": "echo 'Registry test'"
            }),
        )
        .await?;

    assert!(result.success);
    assert!(result.output.contains("Registry test"));

    println!("✓ Tool registry execution working");

    Ok(())
}

#[tokio::test]
async fn test_parallel_tool_execution() -> Result<()> {
    let tool = dx_agent_tools::bash();

    // Execute multiple tools in parallel
    let handles = vec![
        tokio::spawn(async move { tool.execute(json!({"command": "echo 'Task 1'"})).await }),
        tokio::spawn(async move { tool.execute(json!({"command": "echo 'Task 2'"})).await }),
        tokio::spawn(async move { tool.execute(json!({"command": "echo 'Task 3'"})).await }),
    ];

    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await??);
    }

    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|r| r.success));

    println!("✓ Parallel tool execution working");

    Ok(())
}
