//! DX Agent Feature Tests
//!
//! Tests all agent features using Gemini 2.7B model.

use anyhow::Result;
use dx_agent_sessions::{Session, SessionManager};
use dx_agent_tools::{ToolRegistry, bash, read_file, write_file};
use google_gemini_rs::{Client, GenerateContentRequest};

fn load_api_key() -> Result<String> {
    dotenvy::from_filename("../../../.env").ok();
    std::env::var("GOOGLE_API_KEY").map_err(|_| anyhow::anyhow!("GOOGLE_API_KEY not found"))
}

#[tokio::test]
async fn test_session_management() -> Result<()> {
    let manager = SessionManager::new();

    // Create session
    let session_id = manager.create_session("test-user").await?;
    println!("✓ Created session: {}", session_id);

    // Add messages
    manager.add_message(&session_id, "user", "Hello").await?;
    manager.add_message(&session_id, "assistant", "Hi there!").await?;

    // Get history
    let history = manager.get_history(&session_id, 10).await?;
    assert_eq!(history.len(), 2);

    println!("✓ Session management working");

    Ok(())
}

#[tokio::test]
async fn test_tool_registry() -> Result<()> {
    let mut registry = ToolRegistry::new();

    // Register tools
    registry.register(bash());
    registry.register(read_file());
    registry.register(write_file());

    // List tools
    let tools = registry.list();
    assert!(tools.len() >= 3);

    println!("✓ Tool registry working");
    println!("Registered tools: {:?}", tools);

    Ok(())
}

#[tokio::test]
async fn test_agent_with_gemini() -> Result<()> {
    let client = Client::new(&load_api_key()?);
    let manager = SessionManager::new();
    let session_id = manager.create_session("test-user").await?;

    // User message
    let user_msg = "What is 2 + 2?";
    manager.add_message(&session_id, "user", user_msg).await?;

    // Get context
    let history = manager.get_history(&session_id, 10).await?;
    let context = history
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");

    // Call Gemini
    let request = GenerateContentRequest::new("gemini-2.5-flash")
        .with_prompt(&format!("Context:\n{}\n\nRespond to the last message.", context));

    let response = client.generate_content(request).await?;
    let assistant_msg = response.text();

    // Save response
    manager.add_message(&session_id, "assistant", assistant_msg).await?;

    println!("✓ Agent with Gemini working");
    println!("User: {}", user_msg);
    println!("Assistant: {}", assistant_msg);

    assert!(assistant_msg.contains("4") || assistant_msg.contains("four"));

    Ok(())
}

#[tokio::test]
async fn test_context_window_management() -> Result<()> {
    let manager = SessionManager::new();
    let session_id = manager.create_session("test-user").await?;

    // Add many messages
    for i in 0..100 {
        manager.add_message(&session_id, "user", &format!("Message {}", i)).await?;
        manager
            .add_message(&session_id, "assistant", &format!("Response {}", i))
            .await?;
    }

    // Get limited history
    let history = manager.get_history(&session_id, 20).await?;
    assert_eq!(history.len(), 20);

    // Check token budget
    let stats = manager.get_stats(&session_id).await?;
    println!("✓ Context window management working");
    println!("Total messages: {}", stats.total_messages);
    println!("Token count: {}", stats.token_count);

    Ok(())
}

#[tokio::test]
async fn test_session_persistence() -> Result<()> {
    let manager = SessionManager::new();
    let session_id = manager.create_session("test-user").await?;

    // Add messages
    manager.add_message(&session_id, "user", "Test message").await?;

    // Save to disk
    manager.save(&session_id).await?;

    // Load from disk
    let loaded = manager.load(&session_id).await?;
    assert_eq!(loaded.user_id, "test-user");

    println!("✓ Session persistence working");

    Ok(())
}

#[tokio::test]
async fn test_multi_user_sessions() -> Result<()> {
    let manager = SessionManager::new();

    // Create sessions for different users
    let session1 = manager.create_session("user1").await?;
    let session2 = manager.create_session("user2").await?;

    // Add different messages
    manager.add_message(&session1, "user", "User 1 message").await?;
    manager.add_message(&session2, "user", "User 2 message").await?;

    // Verify isolation
    let history1 = manager.get_history(&session1, 10).await?;
    let history2 = manager.get_history(&session2, 10).await?;

    assert_eq!(history1.len(), 1);
    assert_eq!(history2.len(), 1);
    assert_ne!(history1[0].content, history2[0].content);

    println!("✓ Multi-user sessions working");

    Ok(())
}

#[tokio::test]
async fn test_session_compaction() -> Result<()> {
    let manager = SessionManager::new();
    let session_id = manager.create_session("test-user").await?;

    // Add many messages
    for i in 0..50 {
        manager.add_message(&session_id, "user", &format!("Message {}", i)).await?;
    }

    // Compact session
    let removed = manager.compact(&session_id, 20).await?;

    println!("✓ Session compaction working");
    println!("Removed {} old messages", removed);

    let history = manager.get_history(&session_id, 100).await?;
    assert!(history.len() <= 20);

    Ok(())
}
